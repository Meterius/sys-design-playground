use crate::app::map::camera::MapViewCamera;
use crate::app::map::terrain::TerrainTileManager;
use crate::app::map::transform::MERCATOR_WORLD_SIZE;
use crate::utils::mercator_coordinate::{LngLat, MercatorCoordinate};
use bevy::light::CascadeShadowConfigBuilder;
use bevy::prelude::*;
use big_space::bundles::BigSpaceRootBundle;
use big_space::prelude::{CellCoord, FloatingOrigin};
use std::collections::HashMap;

pub(super) struct CorePlugin;

impl Plugin for CorePlugin {
    fn build(&self, _app: &mut App) {}
}

#[derive(Debug, Reflect, Component)]
pub struct MapView {
    pub maplibre_int_id: Entity,
}

pub fn spawn_map_view(commands: &mut Commands, maplibre_integration_id: Entity) {
    let map_view_id = commands
        .spawn((
            Name::new("Map View"),
            BigSpaceRootBundle::default(),
            Visibility::default(),
            MapView {
                maplibre_int_id: maplibre_integration_id,
            },
            TerrainTileManager {
                maplibre_int_id: maplibre_integration_id,
                spawned_tiles: HashMap::default(),
            },
        ))
        .id();

    let world_per_meter = MERCATOR_WORLD_SIZE
        * MercatorCoordinate::from_lng_lat(LngLat::new(13.0, 52.0), 0.0)
            .meter_in_mercator_coordinate_units();

    commands.entity(map_view_id).with_child((
        DirectionalLight {
            color: Color::srgb(0.98, 0.95, 0.82),
            shadows_enabled: true,
            ..default()
        },
        CascadeShadowConfigBuilder {
            first_cascade_far_bound: (world_per_meter * 1000.0) as f32,
            maximum_distance: (world_per_meter * 1000.0) as f32,
            ..default()
        }
        .build(),
        Transform::from_xyz(0.0, 0.0, 0.0).looking_at(Vec3::new(-0.15, -0.05, -0.15), Vec3::Z),
    ));

    commands.entity(map_view_id).with_child((
        Transform::default(),
        CellCoord::default(),
        Camera3d::default(),
        FloatingOrigin,
        MapViewCamera {
            maplibre_int_id: maplibre_integration_id,
        },
    ));
}
