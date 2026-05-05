use bevy::prelude::*;
use std::cell::RefCell;
use std::collections::VecDeque;
use wasm_bindgen::prelude::wasm_bindgen;

use crate::app::map::core::{
    MapView, MapViewCamera, MapViewCameraState, MapViewTileManager, TileKey,
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
        width: f32,
        height: f32,
        zoom: f32,
        pitch: f32,
        bearing: f32,
        center_lng: f32,
        center_lat: f32,
        main_matrix: Vec<f32>,
    },
    SyncTiles {
        canvas_selector: String,
        encoded_tiles: String,
    },
}

#[wasm_bindgen]
pub fn sync_view(
    canvas_selector: String,
    width: f32,
    height: f32,
    zoom: f32,
    pitch: f32,
    bearing: f32,
    center_lng: f32,
    center_lat: f32,
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

                let active_tile_keys = parse_tiles(&encoded_tiles);

                for mut tile_manager in &mut tile_managers {
                    if tile_manager.map_view != Some(map_view_entity) {
                        continue;
                    }

                    tile_manager.active_tile_keys = active_tile_keys.clone();
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

fn parse_tiles(encoded_tiles: &str) -> Vec<TileKey> {
    encoded_tiles
        .split(';')
        .filter_map(|tile| {
            let mut parts = tile.split('/');
            let z = parts.next()?.parse().ok()?;
            let x = parts.next()?.parse().ok()?;
            let y = parts.next()?.parse().ok()?;

            Some(TileKey { z, x, y })
        })
        .collect()
}
