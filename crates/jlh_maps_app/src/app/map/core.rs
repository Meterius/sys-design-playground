use crate::app::common::external_render_target::{
    EXTERNAL_COLOR_TARGET_HANDLE, EXTERNAL_R32F_TARGET_HANDLE,
};
use crate::app::map::buildings::BuildingManager;
use crate::app::map::camera::MapViewCamera;
use crate::app::map::transform::MERCATOR_WORLD_SIZE;
use crate::app::maplibre_gl_js::utils::mercator_coordinate::{LngLat, MercatorCoordinate};
use bevy::camera::RenderTarget;
use bevy::camera::visibility::RenderLayers;
use bevy::prelude::*;
use big_space::bundles::BigSpaceRootBundle;
use big_space::prelude::{CellCoord, FloatingOrigin};
use serde::Serialize;
use std::cell::RefCell;
use std::collections::HashMap;
use wasm_bindgen::prelude::wasm_bindgen;

const FIRST_CASCADE_FAR_METERS: f64 = 3_000.0;
const SHADOW_MAX_DISTANCE_METERS: f64 = 10_000.0;
const SHADOW_MIN_DISTANCE_METERS: f64 = 1.0;
pub const MAP_VIEW_COLOR_RENDER_LAYER: usize = 0;
pub const MAP_VIEW_DEPTH_RENDER_LAYER: usize = 1;

thread_local! {
    static RENDER_TEXTURE_REFERENCES: RefCell<HashMap<(String, u32), MapViewRenderTextureReference>> = RefCell::new(HashMap::new());
}

pub(super) struct CorePlugin;

impl Plugin for CorePlugin {
    fn build(&self, _app: &mut App) {}
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
            // TerrainTileManager {
            //     maplibre_int_id: maplibre_integration_id,
            //     spawned_tiles: HashMap::default(),
            // },
            BuildingManager {
                maplibre_int_id: maplibre_integration_id,
                spawned_buildings: HashMap::default(),
            },
        ))
        .id();

    let world_per_meter = MERCATOR_WORLD_SIZE
        * MercatorCoordinate::from_lng_lat(LngLat::new(13.0, 52.0), 0.0)
            .meter_in_mercator_coordinate_units();
    let _first_cascade_far_bound = (world_per_meter * FIRST_CASCADE_FAR_METERS) as f32;
    let _maximum_distance = (world_per_meter * SHADOW_MAX_DISTANCE_METERS) as f32;
    let _minimum_distance = (world_per_meter * SHADOW_MIN_DISTANCE_METERS) as f32;

    commands.entity(map_view_id).with_child((
        DirectionalLight {
            color: Color::srgb(0.98, 0.95, 0.82),
            shadows_enabled: false,
            shadow_depth_bias: 0.01,
            shadow_normal_bias: 1.8,
            ..default()
        },
        // CascadeShadowConfigBuilder {
        //     num_cascades: 2,
        //     first_cascade_far_bound,
        //     maximum_distance,
        //     minimum_distance,
        //     ..default()
        // }
        // .build(),
        Transform::from_xyz(0.0, 0.0, 0.0).looking_to(Vec3::new(1.0, 0.2, -0.35), Vec3::Z),
        CellCoord::default(),
    ));

    commands.entity(map_view_id).with_child((
        Transform::default(),
        CellCoord::default(),
        Msaa::Off,
        Camera3d::default(),
        Camera {
            clear_color: ClearColorConfig::Custom(Color::NONE),
            ..default()
        },
        FloatingOrigin,
        MapViewCamera {
            maplibre_int_id: maplibre_integration_id,
        },
        RenderLayers::layer(MAP_VIEW_COLOR_RENDER_LAYER),
    ));

    commands.entity(map_view_id).with_child((
        Name::new("External Color Camera"),
        Transform::default(),
        CellCoord::default(),
        Msaa::Off,
        Camera3d::default(),
        Camera {
            clear_color: ClearColorConfig::Custom(Color::NONE),
            order: 1,
            ..default()
        },
        RenderTarget::TextureView(EXTERNAL_COLOR_TARGET_HANDLE),
        RenderLayers::layer(MAP_VIEW_COLOR_RENDER_LAYER),
        MapViewCamera {
            maplibre_int_id: maplibre_integration_id,
        },
    ));

    commands.entity(map_view_id).with_child((
        Name::new("External R32F Probe Camera"),
        Transform::default(),
        CellCoord::default(),
        Msaa::Off,
        Camera3d::default(),
        Camera {
            clear_color: ClearColorConfig::Custom(Color::srgb(0.0, 0.0, 0.0)),
            order: 2,
            ..default()
        },
        RenderTarget::TextureView(EXTERNAL_R32F_TARGET_HANDLE),
        RenderLayers::layer(MAP_VIEW_DEPTH_RENDER_LAYER),
        MapViewCamera {
            maplibre_int_id: maplibre_integration_id,
        },
    ));

    render_texture_reference
}
