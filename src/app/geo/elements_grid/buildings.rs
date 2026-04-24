use crate::app::geo::element_requests::{BuildingRequestClient, BuildingRequestKind};
use crate::app::geo::elements_grid::manager::{
    ElementTileGridConfig, ElementsConfig, ElementsGridPlugin, spawn_elements_grid,
};
use crate::app::geo::geometry_vello::VelloMapLine;
use crate::app::geo::grid::manager::LinearGrid;
use crate::app::utils::async_requests::RequestManager;
use bevy::prelude::*;
use glam::{DVec2, dvec2};
use osm::model::building::Building;
use osm::postgres_integration::client::OsmClient;
use ratelimit::Ratelimiter;
use std::collections::HashMap;
use std::sync::Arc;

pub struct BuildingElementsGridPlugin;

impl Plugin for BuildingElementsGridPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(ElementsGridPlugin::<BuildingRequestKind, BuildingGridKind>::default());
    }
}

struct BuildingClassification {
    kind: BuildingGridKind,
}

fn building_classification(_building: &Building) -> Option<BuildingClassification> {
    Some(BuildingClassification {
        kind: BuildingGridKind::Normal,
    })
}

#[derive(Reflect, Eq, PartialEq, Hash, Debug, Clone, Copy)]
pub enum BuildingGridKind {
    Normal,
}

fn make_building_bundle(
    scene_id: Entity,
    scene_center_abs: DVec2,
    building: &Building,
) -> impl Bundle {
    let geom: Vec<_> = building
        .geometry
        .iter()
        .next()
        .map(|poly| {
            poly.exterior()
                .points()
                .map(|p| dvec2(p.x().to_radians(), p.y().to_radians()))
                .collect()
        })
        .unwrap_or_default();

    assert!(geom.len() >= 2);

    (
        Transform::from_translation(vec3(0.0, 0.0, 1000.0)),
        Name::new("Building"),
        VelloMapLine::new(
            scene_id,
            scene_center_abs,
            geom,
            0.5,
            Color::hsva(38.0, 0.1, 0.4, 1.),
        ),
    )
}

pub fn spawn_building_elements_grid(
    commands: &mut Commands,
    view_id: Entity,
    client: Arc<OsmClient>,
) {
    let make_grid = |count: UVec2, max_spawned: UVec2| -> LinearGrid {
        LinearGrid {
            count,
            active_tile_buffer_using_expansion: uvec2(1, 1),
            active_tile_buffer_using_viewport_extension: vec2(0.2, 0.2),
            min_tile_viewport_percentage: Vec2::ONE / max_spawned.as_vec2(),
        }
    };

    let config = ElementsConfig::<Building, BuildingGridKind> {
        request_grid: make_grid(uvec2(4000, 4000), uvec2(2, 2)),
        tile_grids: HashMap::from([(
            BuildingGridKind::Normal,
            ElementTileGridConfig {
                grid: make_grid(uvec2(8000, 8000), uvec2(2, 2)),
            },
        )]),
        get_tile_grid_for_element: Some(Box::new(|r: &Building| {
            building_classification(r).map(|c| c.kind)
        })),
        on_spawn_element_instance: Some(Box::new(
            |commands, center_abs, tile_id, building_id, building| {
                commands
                    .entity(building_id)
                    .insert(make_building_bundle(tile_id, center_abs, building));
            },
        )),
    };

    let request_manager = RequestManager::new(
        10,
        Some(Ratelimiter::new(20)),
        BuildingRequestClient { client },
    );

    spawn_elements_grid(commands, view_id, config, request_manager);
}
