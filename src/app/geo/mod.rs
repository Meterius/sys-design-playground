use crate::app::geo::geometry::GeometryPlugin;
use crate::app::geo::grid::GridPlugin;
use crate::app::geo::map::MapPlugin;
use crate::app::geo::road_elements::RoadElementsPlugin;
use crate::app::geo::tiling::TilingPlugin;
use crate::app::geo::zoom_visibility::MapZoomVisibilityPlugin;
use bevy::app::App;
use bevy::prelude::Plugin;

mod elements;
pub mod geometry;
pub mod grid;
pub mod map;
pub mod road_elements;
pub mod tiling;
pub mod zoom_visibility;

pub struct GeoPlugin {}

impl Plugin for GeoPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            MapPlugin {},
            GeometryPlugin {},
            TilingPlugin {},
            GridPlugin {},
            MapZoomVisibilityPlugin {},
            RoadElementsPlugin {},
        ));
    }
}
