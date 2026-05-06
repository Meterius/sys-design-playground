use bevy::math::DVec2;
use bevy::prelude::*;
use serde::Deserialize;
use std::cell::RefCell;
use std::collections::VecDeque;
use wasm_bindgen::prelude::wasm_bindgen;

use crate::app::map::core::{
    MapView, MapViewCamera, MapViewCameraState, MapViewTileManager, MapViewTileTexture, Tile,
    TileKey,
};

thread_local! {
    static PENDING_SYNC_COMMANDS: RefCell<VecDeque<MapViewSyncCommand>> = const { RefCell::new(VecDeque::new()) };
}

pub struct MapViewIntegrationPlugin;

impl Plugin for MapViewIntegrationPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PreUpdate, drain_map_view_sync_commands);
    }
}

#[derive(Component)]
pub struct MapViewIntegrationId {
    pub canvas_selector: String,
}

#[derive(Debug)]
enum MapViewSyncCommand {
    SyncView {
        canvas_selector: String,
        width: f64,
        height: f64,
        zoom: f64,
        pitch: f64,
        bearing: f64,
        center_lng: f64,
        center_lat: f64,
        main_matrix: Vec<f64>,
    },
    SyncTiles {
        canvas_selector: String,
        encoded_tiles: String,
    },
    SyncTileTexture {
        canvas_selector: String,
        tile_key: String,
        width: u32,
        height: u32,
        rgba: Vec<u8>,
    },
}

#[wasm_bindgen]
pub fn sync_view(
    canvas_selector: String,
    width: f64,
    height: f64,
    zoom: f64,
    pitch: f64,
    bearing: f64,
    center_lng: f64,
    center_lat: f64,
    main_matrix_json: String,
) {
    let main_matrix = serde_json::from_str(&main_matrix_json).unwrap_or_default();

    enqueue_sync_command(MapViewSyncCommand::SyncView {
        canvas_selector,
        width,
        height,
        zoom,
        pitch,
        bearing,
        center_lng,
        center_lat,
        main_matrix,
    });
}

#[wasm_bindgen]
pub fn sync_tiles(canvas_selector: String, encoded_tiles: String) {
    enqueue_sync_command(MapViewSyncCommand::SyncTiles {
        canvas_selector,
        encoded_tiles,
    });
}

#[wasm_bindgen]
pub fn sync_tile_texture(
    canvas_selector: String,
    tile_key: String,
    width: u32,
    height: u32,
    rgba: Vec<u8>,
) {
    enqueue_sync_command(MapViewSyncCommand::SyncTileTexture {
        canvas_selector,
        tile_key,
        width,
        height,
        rgba,
    });
}

fn enqueue_sync_command(command: MapViewSyncCommand) {
    PENDING_SYNC_COMMANDS.with_borrow_mut(|commands| commands.push_back(command));
}

fn drain_map_view_sync_commands(
    map_views: Query<(Entity, &MapViewIntegrationId), With<MapView>>,
    mut cameras: Query<&mut MapViewCamera>,
    mut tile_managers: Query<&mut MapViewTileManager>,
) {
    let queued_commands =
        PENDING_SYNC_COMMANDS.with_borrow_mut(|pending| pending.drain(..).collect::<Vec<_>>());

    for command in queued_commands {
        match command {
            MapViewSyncCommand::SyncView {
                canvas_selector,
                width,
                height,
                zoom,
                pitch,
                bearing,
                center_lng,
                center_lat,
                main_matrix,
            } => {
                let Some(map_view_entity) = find_map_view(&map_views, &canvas_selector) else {
                    continue;
                };

                for mut camera in &mut cameras {
                    if camera.map_view != Some(map_view_entity) {
                        continue;
                    }

                    camera.state = MapViewCameraState {
                        width,
                        height,
                        zoom,
                        pitch,
                        bearing,
                        center_lng,
                        center_lat,
                        main_matrix: main_matrix.clone(),
                    };
                }
            }
            MapViewSyncCommand::SyncTiles {
                canvas_selector,
                encoded_tiles,
            } => {
                let Some(map_view_entity) = find_map_view(&map_views, &canvas_selector) else {
                    continue;
                };

                let active_tiles = parse_tiles(&encoded_tiles);

                for mut tile_manager in &mut tile_managers {
                    if tile_manager.map_view != Some(map_view_entity) {
                        continue;
                    }

                    tile_manager.active_tiles = active_tiles.clone();
                }
            }
            MapViewSyncCommand::SyncTileTexture {
                canvas_selector,
                tile_key,
                width,
                height,
                rgba,
            } => {
                let Some(map_view_entity) = find_map_view(&map_views, &canvas_selector) else {
                    continue;
                };
                let Some(tile_key) = parse_tile_key(&tile_key) else {
                    continue;
                };

                for mut tile_manager in &mut tile_managers {
                    if tile_manager.map_view != Some(map_view_entity) {
                        continue;
                    }

                    tile_manager.pending_textures.insert(
                        tile_key,
                        MapViewTileTexture {
                            width,
                            height,
                            rgba: rgba.clone(),
                        },
                    );
                }
            }
        }
    }
}

fn find_map_view(
    query: &Query<(Entity, &MapViewIntegrationId), With<MapView>>,
    canvas_selector: &str,
) -> Option<Entity> {
    query
        .iter()
        .find(|(_, integration_id)| integration_id.canvas_selector == canvas_selector)
        .map(|(entity, _)| entity)
}

#[derive(Deserialize)]
struct EncodedTile {
    key: TileKey,
    bounds_lnglat: ([f64; 2], [f64; 2]),
}

fn parse_tiles(encoded_tiles: &str) -> Vec<Tile> {
    serde_json::from_str::<Vec<EncodedTile>>(encoded_tiles)
        .unwrap_or_default()
        .into_iter()
        .map(|tile| Tile {
            key: tile.key,
            bounds_lnglat: (
                DVec2::new(tile.bounds_lnglat.0[0], tile.bounds_lnglat.0[1]),
                DVec2::new(tile.bounds_lnglat.1[0], tile.bounds_lnglat.1[1]),
            ),
        })
        .collect()
}

fn parse_tile_key(tile: &str) -> Option<TileKey> {
    let mut parts = tile.split('/');
    let z = parts.next()?.parse().ok()?;
    let x = parts.next()?.parse().ok()?;
    let y = parts.next()?.parse().ok()?;

    Some(TileKey { z, x, y })
}
