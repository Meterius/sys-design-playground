pub mod buildings;
pub mod camera;
pub mod core;
pub mod interop;
pub mod terrain;
pub mod transform;
pub mod waters;

use bevy::prelude::*;

pub struct MapPlugin;

impl Plugin for MapPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            camera::CameraPlugin,
            core::CorePlugin,
            terrain::TerrainPlugin,
            buildings::BuildingsPlugin,
            waters::WatersPlugin,
        ));
    }
}
