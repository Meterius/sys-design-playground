use crate::app::instance_management::commands::enqueue_instance_command;
use crate::app::map::camera::MapViewCameraSettings as MapViewCameraSettingsBevy;
use crate::app::map::core::MapViewSettings as MapViewSettingsBevy;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen]
pub struct MapViewSettings {
    pub enable_window_cameras: bool,
    pub enable_buildings: bool,
    pub enable_waters: bool,
    pub enable_shadows: bool,
}

#[wasm_bindgen]
impl MapViewSettings {
    #[wasm_bindgen(constructor)]
    pub fn new(
        enable_window_cameras: bool,
        enable_buildings: bool,
        enable_waters: bool,
        enable_shadows: bool,
    ) -> Self {
        Self {
            enable_window_cameras,
            enable_buildings,
            enable_waters,
            enable_shadows,
        }
    }
}

impl From<MapViewSettings> for MapViewSettingsBevy {
    fn from(val: MapViewSettings) -> Self {
        MapViewSettingsBevy {
            enable_buildings: val.enable_buildings,
            enable_waters: val.enable_waters,
            enable_window_cameras: val.enable_window_cameras,
            enable_shadows: val.enable_shadows,
        }
    }
}

#[wasm_bindgen]
pub fn set_map_view_settings(instance_id: String, settings: MapViewSettings) -> Result<(), String> {
    enqueue_instance_command(&instance_id, move |world| {
        *world.get_resource_mut::<MapViewSettingsBevy>().unwrap() = settings.into();
    })
    .map_err(|err| err.to_string())
}

#[wasm_bindgen]
pub struct MapViewCameraSettings {
    pub enable_color_grading: bool,
    pub enable_tonemapping: bool,
    pub enable_msaa: bool,
    pub enable_ssao: bool,
    pub enable_taa: bool,
}

#[wasm_bindgen]
impl MapViewCameraSettings {
    #[wasm_bindgen(constructor)]
    pub fn new(
        enable_color_grading: bool,
        enable_tonemapping: bool,
        enable_msaa: bool,
        enable_ssao: bool,
        enable_taa: bool,
    ) -> Self {
        Self {
            enable_color_grading,
            enable_tonemapping,
            enable_msaa,
            enable_ssao,
            enable_taa,
        }
    }
}

impl From<MapViewCameraSettings> for MapViewCameraSettingsBevy {
    fn from(val: MapViewCameraSettings) -> Self {
        MapViewCameraSettingsBevy {
            enable_color_grading: val.enable_color_grading,
            enable_tonemapping: val.enable_tonemapping,
            enable_msaa: val.enable_msaa,
            enable_ssao: val.enable_ssao,
            enable_taa: val.enable_taa,
        }
    }
}

#[wasm_bindgen]
pub fn set_map_view_camera_settings(
    instance_id: String,
    settings: MapViewCameraSettings,
) -> Result<(), String> {
    enqueue_instance_command(&instance_id, move |world| {
        *world
            .get_resource_mut::<MapViewCameraSettingsBevy>()
            .unwrap() = settings.into();
    })
    .map_err(|err| err.to_string())
}
