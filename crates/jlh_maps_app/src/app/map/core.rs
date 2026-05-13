use crate::app::common::editor::GameViewCamera;
use crate::app::common::external_render_target::EXTERNAL_COLOR_TARGET_HANDLE;
use crate::app::map::buildings::BuildingManager;
use crate::app::map::camera::MapViewCamera;
use crate::app::map::terrain::TerrainTileManager;
use crate::app::map::transform::MERCATOR_WORLD_SIZE;
use crate::app::map::waters::WaterManager;
use crate::app::maplibre_gl_js::utils::mercator_coordinate::{LngLat, MercatorCoordinate};
use bevy::camera::RenderTarget;
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::light::CascadeShadowConfigBuilder;
use bevy::prelude::*;
use bevy::render::view::ColorGrading;
use big_space::bundles::BigSpaceRootBundle;
use big_space::prelude::{CellCoord, FloatingOrigin};
use serde::Serialize;
use std::cell::RefCell;
use std::collections::HashMap;
use wasm_bindgen::prelude::wasm_bindgen;

const FIRST_CASCADE_FAR_METERS: f64 = 3_000.0;
const SHADOW_MAX_DISTANCE_METERS: f64 = 3_000.0;
const SHADOW_MIN_DISTANCE_METERS: f64 = 1.0;
pub const MAP_VIEW_COLOR_RENDER_LAYER: usize = 0;
pub const MAP_VIEW_DEPTH_RENDER_LAYER: usize = 1;

thread_local! {
    static RENDER_TEXTURE_REFERENCES: RefCell<HashMap<(String, u32), MapViewRenderTextureReference>> = RefCell::new(HashMap::new());
}

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
    mut cams: Query<(&mut Camera, &RenderTarget), With<MapViewCamera>>,
) {
    for (mut cam, cam_target) in cams.iter_mut() {
        if matches!(cam_target, RenderTarget::Window(_)) {
            cam.is_active = mv_settings.enable_window_cameras;
        }
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

#[derive(Clone, Debug, Serialize)]
pub struct MapViewRenderTextureReference {
    pub id: u32,
    pub width: u32,
    pub height: u32,
}

#[derive(Component)]
pub struct MapViewRenderTexture {
    pub reference: MapViewRenderTextureReference,
}

#[wasm_bindgen]
pub fn get_map_view_render_texture_reference(
    instance_id: String,
    integration_id: u32,
) -> Result<wasm_bindgen::JsValue, String> {
    RENDER_TEXTURE_REFERENCES.with(|references| {
        references
            .borrow()
            .get(&(instance_id, integration_id))
            .map(serde_wasm_bindgen::to_value)
            .transpose()
            .map_err(|err| err.to_string())?
            .ok_or_else(|| {
                format!("Missing render texture reference for integration {integration_id}")
            })
    })
}

pub fn register_map_view_render_texture_reference(
    instance_id: String,
    integration_id: u32,
    reference: MapViewRenderTextureReference,
) {
    RENDER_TEXTURE_REFERENCES.with(|references| {
        references
            .borrow_mut()
            .insert((instance_id, integration_id), reference);
    });
}

pub fn spawn_map_view(
    commands: &mut Commands,
    maplibre_integration_id: Entity,
    render_texture_reference: MapViewRenderTextureReference,
) -> MapViewRenderTextureReference {
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
            shadow_depth_bias: 0.02,
            shadow_normal_bias: 1.8,
            ..default()
        },
        CascadeShadowConfigBuilder {
            num_cascades: 1,
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

    let tonemapping = Tonemapping::None;
    let msaa = Msaa::Sample8;
    let color_grading = ColorGrading { ..default() };
    let ambient_light = AmbientLight {
        color: Color::WHITE,
        brightness: 1100.0,
        ..default()
    };

    commands.entity(map_view_id).with_child((
        Transform::default(),
        CellCoord::default(),
        Camera3d::default(),
        Camera {
            clear_color: ClearColorConfig::Custom(Color::NONE),
            ..default()
        },
        tonemapping,
        msaa,
        color_grading.clone(),
        ambient_light.clone(),
        GameViewCamera,
        MapViewCamera {
            maplibre_int_id: maplibre_integration_id,
        },
    ));

    commands.entity(map_view_id).with_child((
        Name::new("External Color Camera"),
        Transform::default(),
        CellCoord::default(),
        FloatingOrigin,
        Camera3d::default(),
        Camera {
            clear_color: ClearColorConfig::Custom(Color::NONE),
            ..default()
        },
        tonemapping,
        msaa,
        color_grading,
        ambient_light,
        RenderTarget::TextureView(EXTERNAL_COLOR_TARGET_HANDLE),
        MapViewCamera {
            maplibre_int_id: maplibre_integration_id,
        },
    ));

    render_texture_reference
}
