use crate::app::main::unmount_instance;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen]
pub fn unmount(instance_id: String) -> Result<(), String> {
    unmount_instance(&instance_id)
}
