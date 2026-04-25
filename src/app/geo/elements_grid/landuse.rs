use crate::app::geo::element_requests::{LanduseRequestClient, LanduseRequestKind};
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
use osm::model::landuse::{Landuse, LanduseClass};
use osm::postgres_integration::client::OsmClient;
use ratelimit::Ratelimiter;
use std::collections::HashMap;
use std::sync::Arc;
use bevy_vello::prelude::kurbo;

pub struct LanduseElementsGridPlugin;

impl Plugin for LanduseElementsGridPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(ElementsGridPlugin::<LanduseRequestKind, LanduseGridKind>::default());
    }
}

struct LanduseClassification {
    kind: LanduseGridKind,
}

fn landuse_classification(_landuse: &Landuse) -> Option<LanduseClassification> {
    Some(LanduseClassification {
        kind: LanduseGridKind::Normal,
    })
}

#[derive(Reflect, Eq, PartialEq, Hash, Debug, Clone, Copy)]
pub enum LanduseGridKind {
    Normal,
}

fn make_landuse_bundle(scene_id: Entity, scene_center_abs: DVec2, landuse: &Landuse) -> impl Bundle {
    let geom = MultiPolygon::from_iter(landuse.geometry.iter().map(|poly| {
        Polygon::new(
            LineString::from_iter(poly.exterior().points().map(|p| p.to_radians())),
            poly.interiors()
                .iter()
                .map(|interior| LineString::from_iter(interior.points().map(|p| p.to_radians())))
                .collect(),
        )
    }));

    let color = match &landuse.class {
        LanduseClass::Forest => Color::hsv(120.0, 0.5, 0.3),
        LanduseClass::Grass => Color::hsv(120.0, 0.5, 0.7),
        LanduseClass::Park => Color::hsv(120.0, 0.5, 0.5),
        _ => Color::hsv(120.0, 0.0, 0.9),
    };

    (
        Transform::from_translation(vec3(0.0, 0.0, 800.0)),
        Name::new("Landuse"),
        VelloMapPolygon::new(
            scene_id,
            0,
            scene_center_abs,
            geom,
            color,
            Some((
                color.darker(0.5),
                kurbo::Stroke::new(0.5),
            )),
        ),
    )
}

pub fn spawn_landuse_elements_grid(commands: &mut Commands, view_id: Entity, client: Arc<OsmClient>) {
    let make_grid = |count: UVec2, max_spawned: UVec2| -> LinearGrid {
        LinearGrid {
            count,
            active_tile_buffer_using_expansion: uvec2(1, 1),
            active_tile_buffer_using_viewport_extension: vec2(0.2, 0.2),
            min_tile_viewport_percentage: Vec2::ONE / max_spawned.as_vec2(),
        }
    };

    let config = ElementsConfig::<Landuse, LanduseGridKind> {
        request_grid: make_grid(uvec2(1000, 1000), uvec2(2, 2)),
        tile_grids: HashMap::from([(
            LanduseGridKind::Normal,
            ElementTileGridConfig {
                grid: make_grid(uvec2(2000, 2000), uvec2(2, 2)),
            },
        )]),
        get_tile_grid_for_element: Some(Box::new(|r: &Landuse| {
            landuse_classification(r).map(|c| c.kind)
        })),
        on_spawn_element_instance: Some(Box::new(
            |commands, center_abs, tile_id, landuse_id, landuse| {
                commands
                    .entity(landuse_id)
                    .insert(make_landuse_bundle(tile_id, center_abs, landuse));
            },
        )),
        on_spawn_tile: None,
    };

    let request_manager = RequestManager::new(
        10,
        Some(Ratelimiter::new(20)),
        LanduseRequestClient { client },
    );

    spawn_elements_grid(commands, view_id, config, request_manager);
}
