use crate::app::geo::element_requests::{WaterRequestClient, WaterRequestKind};
use crate::app::geo::elements_grid::manager::{
    ElementTileGridConfig, ElementsConfig, ElementsGridPlugin, spawn_elements_grid,
};
use crate::app::geo::geometry_vello::VelloMapPolygon;
use crate::app::geo::grid::manager::LinearGrid;
use crate::app::utils::async_requests::RequestManager;
use bevy::prelude::*;
use geo_types::geometry::Polygon;
use geo_types::{LineString, MultiPolygon};
use glam::DVec2;
use osm::model::water::Water;
use osm::postgres_integration::client::OsmClient;
use ratelimit::Ratelimiter;
use std::collections::HashMap;
use std::sync::Arc;
use bevy_vello::prelude::kurbo;

pub struct WaterElementsGridPlugin;

impl Plugin for WaterElementsGridPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(ElementsGridPlugin::<WaterRequestKind, WaterGridKind>::default());
    }
}

struct WaterClassification {
    kind: WaterGridKind,
}

fn water_classification(_water: &Water) -> Option<WaterClassification> {
    Some(WaterClassification {
        kind: WaterGridKind::Normal,
    })
}

#[derive(Reflect, Eq, PartialEq, Hash, Debug, Clone, Copy)]
pub enum WaterGridKind {
    Normal,
}

fn make_water_bundle(scene_id: Entity, scene_center_abs: DVec2, water: &Water) -> impl Bundle {
    let geom = MultiPolygon::from_iter(water.geometry.iter().map(|poly| {
        Polygon::new(
            LineString::from_iter(poly.exterior().points().map(|p| p.to_radians())),
            poly.interiors()
                .iter()
                .map(|interior| LineString::from_iter(interior.points().map(|p| p.to_radians())))
                .collect(),
        )
    }));

    (
        Transform::from_translation(vec3(0.0, 0.0, 800.0)),
        Name::new("Water"),
        VelloMapPolygon::new(
            scene_id,
            0,
            scene_center_abs,
            geom,
            Color::hsva(194.0, 0.7, 0.8, 1.),
            Some((
                Color::hsva(194.0, 0.7, 0.8, 1.),
                kurbo::Stroke::new(0.5),
            )),
        ),
    )
}

pub fn spawn_water_elements_grid(commands: &mut Commands, view_id: Entity, client: Arc<OsmClient>) {
    let make_grid = |count: UVec2, max_spawned: UVec2| -> LinearGrid {
        LinearGrid {
            count,
            active_tile_buffer_using_expansion: uvec2(1, 1),
            active_tile_buffer_using_viewport_extension: vec2(0.2, 0.2),
            min_tile_viewport_percentage: Vec2::ONE / max_spawned.as_vec2(),
        }
    };

    let config = ElementsConfig::<Water, WaterGridKind> {
        request_grid: make_grid(uvec2(1000, 1000), uvec2(2, 2)),
        tile_grids: HashMap::from([(
            WaterGridKind::Normal,
            ElementTileGridConfig {
                grid: make_grid(uvec2(2000, 2000), uvec2(2, 2)),
            },
        )]),
        get_tile_grid_for_element: Some(Box::new(|r: &Water| {
            water_classification(r).map(|c| c.kind)
        })),
        on_spawn_element_instance: Some(Box::new(
            |commands, center_abs, tile_id, water_id, water| {
                commands
                    .entity(water_id)
                    .insert(make_water_bundle(tile_id, center_abs, water));
            },
        )),
        on_spawn_tile: None,
    };

    let request_manager = RequestManager::new(
        10,
        Some(Ratelimiter::new(20)),
        WaterRequestClient { client },
    );

    spawn_elements_grid(commands, view_id, config, request_manager);
}
