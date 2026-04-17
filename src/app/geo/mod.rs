use crate::app::geo::geometry::GeometryPlugin;
use crate::app::geo::map::MapPlugin;
use crate::app::geo::tiling::TilingPlugin;
use bevy::app::App;
use bevy::prelude::Plugin;

pub mod geometry;
pub mod map;
pub mod tiling;

pub struct GeoPlugin {}

impl Plugin for GeoPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            MapPlugin {},
            GeometryPlugin {},
            TilingPlugin {},
        ));
    }
}
