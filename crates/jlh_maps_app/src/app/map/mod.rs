pub mod buildings;
pub mod camera;
pub mod core;
mod interop;
pub mod terrain;
pub mod transform;

use bevy::prelude::*;

pub struct MapPlugin;

impl Plugin for MapPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            camera::CameraPlugin,
            core::CorePlugin,
            terrain::TerrainPlugin,
            buildings::BuildingsPlugin,
        ));
    }
}
