use bevy::app::Plugin;
use crate::app::geo::tiling::manager::TilingMangerPlugin;
use crate::app::geo::tiling::requests::TilingRequestPlugin;
use crate::app::geo::tiling::sprite::TilingSpritePlugin;

pub mod manager;
mod sprite;
mod requests;

pub struct TilingPlugin {}

impl Plugin for TilingPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_plugins((
            TilingRequestPlugin {},
            TilingSpritePlugin {},
            TilingMangerPlugin {},
        ));
    }
}