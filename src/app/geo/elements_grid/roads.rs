use crate::app::geo::element_requests::{RoadRequestClient, RoadRequestKind};
use crate::app::geo::elements_grid::manager::{
    ElementTileGridConfig, ElementsConfig, ElementsGridPlugin, spawn_elements_grid,
};
use crate::app::geo::geometry_vello::VelloMapLine;
use crate::app::geo::grid::manager::LinearGrid;
use crate::app::utils::async_requests::RequestManager;
use crate::app::utils::vello_ext::VelloEnhancedScene;
use bevy::prelude::*;
use bevy_vello::prelude::{VelloScene2d, peniko};
use glam::{DVec2, dvec2};
use osm::model::road::{Road, RoadClass, RoadClassCategory};
use osm::postgres_integration::client::OsmClient;
use ratelimit::Ratelimiter;
use std::collections::HashMap;
use std::sync::Arc;

pub struct RoadElementsGridPlugin;

impl Plugin for RoadElementsGridPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(ElementsGridPlugin::<RoadRequestKind, RoadGridKind>::default());
    }
}

struct RoadClassification {
    width: f32,
    kind: RoadGridKind,
}

fn road_classification(road: &Road) -> Option<RoadClassification> {
    let kind = match (road.class.category(), road.class) {
        (_, RoadClass::Primary | RoadClass::PrimaryLink) => RoadGridKind::Large,
        (_, RoadClass::Motorway | RoadClass::MotorwayLink) => RoadGridKind::Large,
        (_, RoadClass::Trunk | RoadClass::TrunkLink) => RoadGridKind::Large,

        (RoadClassCategory::HighwayLinks, _) => RoadGridKind::Medium,
        (RoadClassCategory::MajorRoads, _) => RoadGridKind::Medium,
        (RoadClassCategory::MinorRoads, _) => RoadGridKind::Medium,

        (RoadClassCategory::VerySmallRoads, _) => RoadGridKind::Small,
        (RoadClassCategory::PathsUnsuitableForCars, _) => RoadGridKind::Small,
        (RoadClassCategory::Unknown, _) => RoadGridKind::Small,
        // _ => return None,
    };

    let width = match (road.class.category(), road.class) {
        (_, RoadClass::Primary | RoadClass::PrimaryLink) => 6.0,
        (_, RoadClass::Motorway | RoadClass::MotorwayLink) => 6.0,
        (_, RoadClass::Trunk | RoadClass::TrunkLink) => 6.0,

        (RoadClassCategory::HighwayLinks, _) => 3.0,
        (RoadClassCategory::MajorRoads, _) => 3.0,

        (RoadClassCategory::MinorRoads, _) => 1.0,

        (RoadClassCategory::VerySmallRoads, _) => 0.2,
        (RoadClassCategory::PathsUnsuitableForCars, _) => 0.25,
        (RoadClassCategory::Unknown, _) => 0.1,
    };

    Some(RoadClassification { width, kind })
}

#[derive(Reflect, Eq, PartialEq, Hash, Debug, Clone, Copy)]
pub enum RoadGridKind {
    Small,
    Medium,
    Large,
}

fn make_road_bundle(scene_id: Entity, scene_center_abs: DVec2, road: &Road) -> impl Bundle {
    (
        Transform::from_translation(vec3(0.0, 0.0, 1000.0)),
        Name::new("Road"),
        // Visibility::Visible,
        // MapZoomVisibility {
        //     visible_abs_view_perc: (
        //         0.0,
        //         0.00075
        //             * match road.class.category() {
        //                 RoadClassCategory::HighwayLinks => 3.0,
        //                 RoadClassCategory::MajorRoads => 3.0,
        //                 RoadClassCategory::MinorRoads => 0.85,
        //                 RoadClassCategory::Unknown => 0.25,
        //                 RoadClassCategory::VerySmallRoads => 0.25,
        //                 RoadClassCategory::PathsUnsuitableForCars => 0.25,
        //             },
        //     ),
        // },
    )
}

pub fn spawn_road_elements_grid(commands: &mut Commands, view_id: Entity, client: Arc<OsmClient>) {
    let make_grid = |count: UVec2, max_spawned: UVec2| -> LinearGrid {
        LinearGrid {
            count,
            active_tile_buffer_using_expansion: uvec2(1, 1),
            active_tile_buffer_using_viewport_extension: vec2(0.2, 0.2),
            min_tile_viewport_percentage: Vec2::ONE / max_spawned.as_vec2(),
        }
    };

    let config = ElementsConfig::<Road, RoadGridKind> {
        request_grid: make_grid(uvec2(1000, 1000), uvec2(4, 4)),
        tile_grids: HashMap::from([
            (
                RoadGridKind::Large,
                ElementTileGridConfig {
                    grid: make_grid(uvec2(1000, 1000), uvec2(2, 2)),
                },
            ),
            (
                RoadGridKind::Medium,
                ElementTileGridConfig {
                    grid: make_grid(uvec2(2000, 2000), uvec2(2, 2)),
                },
            ),
            (
                RoadGridKind::Small,
                ElementTileGridConfig {
                    grid: make_grid(uvec2(24000, 24000), uvec2(2, 2)),
                },
            ),
        ]),
        get_tile_grid_for_element: Some(Box::new(|r: &Road| {
            road_classification(r).map(|c| c.kind)
        })),
        on_spawn_element_instance: Some(Box::new(
            |commands, center_abs, tile_id, road_id, road| {
                commands
                    .entity(road_id)
                    .insert(make_road_bundle(tile_id, center_abs, road));

                commands.entity(road_id).with_children(|commands| {
                    commands.spawn((
                        Transform::default(),
                        VelloMapLine::new(
                            tile_id,
                            0,
                            center_abs,
                            road.geometry
                                .iter()
                                .map(|pos| dvec2(pos.x.to_radians(), pos.y.to_radians()))
                                .collect(),
                            road_classification(road).map(|c| c.width).unwrap_or(1.) + 0.1,
                            Color::hsva(38.0, 0.0, 0.3, 1.),
                        ),
                    ));

                    commands.spawn((
                        Transform::default(),
                        VelloMapLine::new(
                            tile_id,
                            1,
                            center_abs,
                            road.geometry
                                .iter()
                                .map(|pos| dvec2(pos.x.to_radians(), pos.y.to_radians()))
                                .collect(),
                            road_classification(road).map(|c| c.width).unwrap_or(1.),
                            Color::hsva(38.0, 0.0, 0.7, 1.),
                        ),
                    ));
                });
            },
        )),
        on_spawn_tile: Some(Arc::new(|commands, ctx, tile_id, tile| {
            commands.entity(tile_id).insert(VelloEnhancedScene {
                on_layer_draw_begin: HashMap::from([
                        // (1, Box::new(move |scene: &mut VelloScene2d| {
                        //     scene.push_layer(
                        //         peniko::Fill::default(),
                        //         peniko::BlendMode {
                        //             compose: peniko::Compose::SrcOver,
                        //             mix: peniko::Mix::Normal,
                        //         }
                        //     )
                        // }))
                    ]),
            });
        })),
    };

    let request_manager =
        RequestManager::new(10, Some(Ratelimiter::new(20)), RoadRequestClient { client });

    spawn_elements_grid(commands, view_id, config, request_manager);
}
