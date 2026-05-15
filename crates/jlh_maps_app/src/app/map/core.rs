use crate::app::common::editor::GameViewCamera;
use crate::app::main::AppWindows;
use crate::app::map::buildings::BuildingManager;
use crate::app::map::camera::MapViewCamera;
use crate::app::map::terrain::TerrainTileManager;
use crate::app::map::transform::MERCATOR_WORLD_SIZE;
use crate::app::map::waters::WaterManager;
use crate::app::maplibre_gl_js::utils::mercator_coordinate::{LngLat, MercatorCoordinate};
use bevy::camera::{CameraOutputMode, RenderTarget};
use bevy::light::CascadeShadowConfigBuilder;
use bevy::prelude::*;
use bevy::window::WindowRef;
use big_space::bundles::BigSpaceRootBundle;
use big_space::prelude::{CellCoord, FloatingOrigin};
use std::collections::HashMap;

const FIRST_CASCADE_FAR_METERS: f64 = 2_000.0;
const SHADOW_MAX_DISTANCE_METERS: f64 = 10_000.0;
const SHADOW_MIN_DISTANCE_METERS: f64 = 1.0;
const SHADOW_DEPTH_BIAS: f32 = 0.01;
const SHADOW_NORMAL_BIAS: f32 = 1.8;
pub const MAP_VIEW_COLOR_RENDER_LAYER: usize = 0;
pub const MAP_VIEW_DEPTH_RENDER_LAYER: usize = 1;

pub(super) struct CorePlugin;

impl Plugin for CorePlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(MapViewSettings {
            enable_window_cameras: false,
            enable_waters: true,
            enable_buildings: true,
            enable_shadows: true,
        });

        app.add_systems(PreUpdate, (sync_window_cameras, sync_map_shadows));
    }
}

#[derive(Debug, Reflect, Resource)]
pub struct MapViewSettings {
    pub enable_window_cameras: bool,

    pub enable_buildings: bool,
    pub enable_waters: bool,
    pub enable_shadows: bool,
}

#[derive(Debug, Reflect, Component)]
struct MapViewShadowLight;

fn sync_window_cameras(
    mv_settings: Res<MapViewSettings>,
    mut cams: Query<&mut Camera, With<GameViewCamera>>,
) {
    for mut cam in cams.iter_mut() {
        cam.is_active = mv_settings.enable_window_cameras;
    }
}

fn sync_map_shadows(
    mv_settings: Res<MapViewSettings>,
    mut lights: Query<&mut DirectionalLight, With<MapViewShadowLight>>,
) {
    for mut light in lights.iter_mut() {
        light.shadows_enabled = mv_settings.enable_shadows;
    }
}

#[derive(Debug, Reflect, Component)]
pub struct MapView {
    pub maplibre_int_id: Entity,
}

pub fn spawn_map_view(
    commands: &mut Commands,
    maplibre_integration_id: Entity,
    app_windows: &AppWindows,
) {
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
            BuildingManager {
                maplibre_int_id: maplibre_integration_id,
                spawned_buildings: HashMap::default(),
            },
            WaterManager {
                maplibre_int_id: maplibre_integration_id,
                spawned_waters: HashMap::default(),
            },
        ))
        .id();

    let world_per_meter = MERCATOR_WORLD_SIZE
        * MercatorCoordinate::from_lng_lat(LngLat::new(13.0, 52.0), 0.0)
            .meter_in_mercator_coordinate_units();
    let first_cascade_far_bound = (world_per_meter * FIRST_CASCADE_FAR_METERS) as f32;
    let maximum_distance = (world_per_meter * SHADOW_MAX_DISTANCE_METERS) as f32;
    let minimum_distance = (world_per_meter * SHADOW_MIN_DISTANCE_METERS) as f32;

    commands.entity(map_view_id).with_child((
        DirectionalLight {
            color: Color::WHITE,
            illuminance: 4000.,
            shadows_enabled: true,
            shadow_depth_bias: SHADOW_DEPTH_BIAS,
            shadow_normal_bias: SHADOW_NORMAL_BIAS,
            ..default()
        },
        CascadeShadowConfigBuilder {
            num_cascades: 3,
            first_cascade_far_bound,
            maximum_distance,
            minimum_distance,
            ..default()
        }
        .build(),
        Transform::default().looking_to(Vec3::new(1.0, 0.2, -0.65), Vec3::Z),
        CellCoord::default(),
        MapViewShadowLight,
    ));

    let ambient_light = AmbientLight {
        color: Color::WHITE,
        brightness: 1100.0,
        ..default()
    };

    commands.entity(map_view_id).with_children(|parent| {
        parent.spawn((
            Transform::default(),
            CellCoord::default(),
            Camera3d::default(),
            Camera {
                clear_color: ClearColorConfig::Custom(Color::NONE),
                output_mode: CameraOutputMode::Write {
                    clear_color: ClearColorConfig::Custom(Color::NONE),
                    blend_state: None,
                },
                ..default()
            },
            ambient_light.clone(),
            RenderTarget::Window(WindowRef::Entity(
                app_windows.debug.expect("debug offscreen window to be set"),
            )),
            GameViewCamera,
            MapViewCamera {
                maplibre_int_id: maplibre_integration_id,
            },
        ));
    });

    commands.entity(map_view_id).with_children(|parent| {
        parent.spawn((
            Name::new("MapLibre Texture Camera"),
            Transform::default(),
            CellCoord::default(),
            Camera3d::default(),
            FloatingOrigin,
            Camera {
                clear_color: ClearColorConfig::Custom(Color::NONE),
                output_mode: CameraOutputMode::Write {
                    clear_color: ClearColorConfig::Custom(Color::NONE),
                    blend_state: None,
                },
                ..default()
            },
            RenderTarget::Window(WindowRef::Entity(
                app_windows
                    .texture
                    .expect("map texture offscreen window to be set"),
            )),
            ambient_light,
            MapViewCamera {
                maplibre_int_id: maplibre_integration_id,
            },
        ));
    });
}
