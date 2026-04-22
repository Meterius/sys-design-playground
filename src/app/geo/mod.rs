use crate::app::geo::element_requests::{ElementRequestsPlugin, RoadRequestKind};
use crate::app::geo::elements_grid::{ElementsGridPlugin, RoadGridKind};
use crate::app::geo::geometry::GeometryPlugin;
use crate::app::geo::grid::GridPlugin;
use crate::app::geo::map::MapPlugin;
use crate::app::geo::tiling::TilingPlugin;
use crate::app::geo::zoom_visibility::MapZoomVisibilityPlugin;
use bevy::app::App;
use bevy::prelude::Plugin;

pub mod element_requests;
pub mod elements_grid;
pub mod geometry;
pub mod grid;
pub mod map;
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
            ElementRequestsPlugin {},
            ElementsGridPlugin::<RoadRequestKind, RoadGridKind>::default(),
        ));
    }
}
