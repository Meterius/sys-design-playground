use crate::app::geo::element_requests::{BuildingRequestClient, BuildingRequestKind};
use crate::app::geo::elements_grid::manager::{
    ElementTileGridConfig, ElementsConfig, ElementsGridPlugin, spawn_elements_grid,
};
use crate::app::geo::geometry_vello::{VelloMapLine, VelloMapPolygon};
use crate::app::geo::grid::manager::LinearGrid;
use crate::app::utils::async_requests::RequestManager;
use bevy::prelude::*;
use glam::{DVec2, dvec2};
use osm::model::building::Building;
use osm::postgres_integration::client::OsmClient;
use ratelimit::Ratelimiter;
use std::collections::HashMap;
use std::sync::Arc;
use geo_types::{LineString, MultiPolygon};
use bevy_vello::prelude::kurbo;

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
    let geom = MultiPolygon::from_iter(building.geometry.iter().map(|poly| {
        geo_types::Polygon::new(
            LineString::from_iter(poly.exterior().points().map(|p| p.to_radians())),
            poly.interiors()
                .iter()
                .map(|interior| LineString::from_iter(interior.points().map(|p| p.to_radians())))
                .collect(),
        )
    }));

    (
        Transform::from_translation(vec3(0.0, 0.0, 1000.0)),
        Name::new("Building"),
        VelloMapPolygon::new(
            scene_id,
            0,
            scene_center_abs,
            geom,
            Color::hsva(38.0, 0.1, 0.6, 1.),
            Some((
                Color::hsva(38.0, 0.1, 0.3, 1.),
                kurbo::Stroke::new(0.075),
            )),
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
        on_spawn_tile: None,
    };

    let request_manager = RequestManager::new(
        10,
        Some(Ratelimiter::new(20)),
        BuildingRequestClient { client },
    );

    spawn_elements_grid(commands, view_id, config, request_manager);
}
