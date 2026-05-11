use crate::app::instance_management::commands::enqueue_instance_command;
use crate::app::map::core::MapViewSettings;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen]
pub fn set_map_view_enable_window_cameras(instance_id: String, enable: bool) -> Result<(), String> {
    enqueue_instance_command(&instance_id, move |world| {
        world
            .get_resource_mut::<MapViewSettings>()
            .unwrap()
            .enable_window_cameras = enable;
    })
    .map_err(|err| err.to_string())
}
