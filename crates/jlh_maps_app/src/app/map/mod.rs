pub mod camera;
pub mod core;
pub mod transform;

use bevy::prelude::*;

pub struct MapPlugin;

impl Plugin for MapPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((camera::CameraPlugin, core::CorePlugin));
    }
}
