use crate::app::geo::map::MapPlugin;
use crate::app::geo::tile_fetcher::TileFetcherPlugin;
use crate::app::geo::tiling::MapViewTilingPlugin;
use bevy::app::App;
use bevy::prelude::Plugin;
use crate::app::geo::geometry::GeometryPlugin;

pub mod map;
mod tile_fetcher;
pub mod tiling;
pub mod geometry;

pub struct GeoPlugin {}

impl Plugin for GeoPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            MapPlugin {},
            TileFetcherPlugin {},
            MapViewTilingPlugin {},
            GeometryPlugin {},
        ));
    }
}
