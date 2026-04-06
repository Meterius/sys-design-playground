use crate::app::geo::map::MapPlugin;
use crate::app::geo::tiling::MapViewTilingPlugin;
use bevy::app::App;
use bevy::prelude::Plugin;

pub mod map;
pub mod tiling;

pub struct GeoPlugin {}

impl Plugin for GeoPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((MapPlugin {}, MapViewTilingPlugin {}));
    }
}
