use crate::app::geo::map::MapPlugin;
use crate::app::geo::osm::OsmPlugin;
use crate::app::geo::tile_fetcher::TileFetcherPlugin;
use crate::app::geo::tiling::MapViewTilingPlugin;
use bevy::app::App;
use bevy::prelude::Plugin;

pub mod map;
pub mod osm;
mod tile_fetcher;
pub mod tiling;

pub struct GeoPlugin {}

impl Plugin for GeoPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            MapPlugin {},
            TileFetcherPlugin {},
            MapViewTilingPlugin {},
            OsmPlugin {},
        ));
    }
}
