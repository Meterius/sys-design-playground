pub mod integration;
pub mod interop;
pub mod types;
pub mod utils;

use bevy::prelude::*;

pub struct MaplibreGlJsPlugin;

impl Plugin for MaplibreGlJsPlugin {
    fn build(&self, _app: &mut App) {}
}
