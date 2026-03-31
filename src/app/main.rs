#[cfg(target_family = "wasm")]
extern crate wasm_bindgen;

#[cfg(target_family = "wasm")]
use wasm_bindgen::prelude::*;

#[cfg(target_family = "wasm")]
#[wasm_bindgen]
extern {
    fn alert(s: &str);
}

#[cfg(target_family = "wasm")]
#[wasm_bindgen]
pub fn initialize() {
    alert("hello world");
}