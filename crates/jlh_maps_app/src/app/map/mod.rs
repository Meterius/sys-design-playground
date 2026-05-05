pub mod core;
pub mod integration;

use bevy::prelude::*;

use crate::app::map::core::MapViewCorePlugin;
use crate::app::map::integration::MapViewIntegrationPlugin;

pub struct MapViewPlugin;

impl Plugin for MapViewPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((MapViewCorePlugin, MapViewIntegrationPlugin));
    }
}
