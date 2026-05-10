use crate::app::instance_management::commands::enqueue_instance_command;
use bevy::app::AppExit;
use tracing::info;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen]
pub fn unmount(instance_id: String) -> Result<(), String> {
    enqueue_instance_command(&instance_id.clone(), move |world| {
        info!("Unmounting instance {}", instance_id);
        world.write_message(AppExit::Success);
    })
    .map_err(|err| err.to_string())
}
