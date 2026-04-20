use crate::app::geo::tiling::manager::setup_tiles;
use crate::app::geo::tiling::requests::TileImageRequestKind;
use crate::app::utils::async_requests::{Request, RequestState};
use bevy::prelude::*;

#[derive(Default)]
pub struct TilingSpritePlugin {}

impl Plugin for TilingSpritePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, handle_tile_image_sprite_loaded.after(setup_tiles));
    }
}

#[derive(Component)]
pub struct TileImageSprite {
    pub size: Option<Vec2>,
}

pub fn handle_tile_image_sprite_loaded(
    mut commands: Commands,
    tiles: Query<
        (Entity, &TileImageSprite, &Request<TileImageRequestKind>),
        Changed<Request<TileImageRequestKind>>,
    >,
    asset_server: Res<AssetServer>,
) {
    for (tile_id, tile_sprite, tile_res) in tiles {
        if let RequestState::Completed(Ok(Some(path))) = tile_res.state() {
            commands.entity(tile_id).insert(Sprite {
                image: asset_server.load(path.as_os_str().to_str().unwrap().to_owned()),
                custom_size: tile_sprite.size,
                ..default()
            });
        }
    }
}
