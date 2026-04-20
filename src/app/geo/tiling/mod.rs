use crate::app::geo::tiling::manager::TilingMangerPlugin;
use crate::app::geo::tiling::requests::TilingRequestPlugin;
use crate::app::geo::tiling::sprite::TilingSpritePlugin;
use bevy::app::Plugin;

pub mod manager;
mod requests;
mod sprite;

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
