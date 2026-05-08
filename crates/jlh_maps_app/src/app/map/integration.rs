use bevy::math::{DMat4, DVec2};
use bevy::prelude::*;
use serde::Deserialize;
use std::cell::RefCell;
use std::collections::VecDeque;
use wasm_bindgen::prelude::wasm_bindgen;

use crate::app::map::core::{
    MapView, MapViewCamera, MapViewCameraState, MapViewFeatureData, MapViewFeatureKey,
    MapViewFeatureManager, MapViewTileManager, MapViewTileTerrainData, Tile, TileKey,
};
use crate::utils::dem_data::DEMData;
use crate::utils::terrain::TerrainData;
use geojson::Geometry;

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
    View {
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
    Tiles {
        canvas_selector: String,
        encoded_tiles: String,
    },
    Features {
        canvas_selector: String,
        encoded_features: String,
    },
    TerrainData {
        canvas_selector: String,
        tile_key: String,
        content_stamp: String,
        stride: u32,
        dim: u32,
        min: f64,
        max: f64,
        red_factor: f64,
        green_factor: f64,
        blue_factor: f64,
        base_shift: f64,
        terrain_matrix: DMat4,
        data: Vec<u32>,
    },
}

#[allow(clippy::too_many_arguments)]
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

    enqueue_sync_command(MapViewSyncCommand::View {
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
    enqueue_sync_command(MapViewSyncCommand::Tiles {
        canvas_selector,
        encoded_tiles,
    });
}

#[wasm_bindgen]
pub fn sync_features(canvas_selector: String, encoded_features: String) {
    enqueue_sync_command(MapViewSyncCommand::Features {
        canvas_selector,
        encoded_features,
    });
}

#[wasm_bindgen]
#[allow(clippy::too_many_arguments)]
pub fn sync_terrain_data(
    canvas_selector: String,
    tile_key: String,
    content_stamp: String,
    stride: u32,
    dim: u32,
    min: f64,
    max: f64,
    red_factor: f64,
    green_factor: f64,
    blue_factor: f64,
    base_shift: f64,
    terrain_matrix_json: String,
    data: Vec<u32>,
) {
    let terrain_matrix = serde_json::from_str::<Vec<f64>>(&terrain_matrix_json).unwrap_or_default();
    let terrain_matrix = DMat4::from_cols_array(terrain_matrix.as_slice().try_into().unwrap());

    enqueue_sync_command(MapViewSyncCommand::TerrainData {
        canvas_selector,
        tile_key,
        content_stamp,
        stride,
        dim,
        min,
        max,
        red_factor,
        green_factor,
        blue_factor,
        base_shift,
        terrain_matrix,
        data,
    });
}

fn enqueue_sync_command(command: MapViewSyncCommand) {
    PENDING_SYNC_COMMANDS.with_borrow_mut(|commands| commands.push_back(command));
}

fn drain_map_view_sync_commands(
    map_views: Query<(Entity, &MapViewIntegrationId), With<MapView>>,
    mut cameras: Query<&mut MapViewCamera>,
    mut tile_managers: Query<&mut MapViewTileManager>,
    mut feature_managers: Query<&mut MapViewFeatureManager>,
) {
    let queued_commands =
        PENDING_SYNC_COMMANDS.with_borrow_mut(|pending| pending.drain(..).collect::<Vec<_>>());

    for command in queued_commands {
        match command {
            MapViewSyncCommand::View {
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
            MapViewSyncCommand::Tiles {
                canvas_selector,
                encoded_tiles,
            } => {
                let Some(map_view_id) = find_map_view(&map_views, &canvas_selector) else {
                    continue;
                };

                let active_tiles = parse_tiles(&encoded_tiles);

                for mut tile_manager in &mut tile_managers {
                    if tile_manager.map_view_id != map_view_id {
                        continue;
                    }

                    tile_manager.active_tiles = active_tiles.clone();
                }
            }
            MapViewSyncCommand::Features {
                canvas_selector,
                encoded_features,
            } => {
                let Some(map_view_id) = find_map_view(&map_views, &canvas_selector) else {
                    continue;
                };

                let active_features = parse_features(&encoded_features);

                for mut feature_manager in &mut feature_managers {
                    if feature_manager.map_view_id != map_view_id {
                        continue;
                    }

                    feature_manager.active_features = active_features.clone();
                }
            }
            MapViewSyncCommand::TerrainData {
                canvas_selector,
                tile_key,
                content_stamp,
                stride,
                dim,
                min,
                max,
                red_factor,
                green_factor,
                blue_factor,
                base_shift,
                terrain_matrix,
                data,
            } => {
                let Some(map_view_id) = find_map_view(&map_views, &canvas_selector) else {
                    continue;
                };
                let Some(tile_key) = parse_tile_key(&tile_key) else {
                    continue;
                };

                if let Some(mut tile_manager) = tile_managers
                    .iter_mut()
                    .filter(|tile_manager| tile_manager.map_view_id == map_view_id)
                    .next()
                {
                    tile_manager.terrain_data.insert(
                        tile_key,
                        MapViewTileTerrainData {
                            content_stamp,
                            terrain_data: TerrainData {
                                dem_data: DEMData {
                                    data,
                                    stride,
                                    dim,
                                    min,
                                    max,
                                    red_factor,
                                    green_factor,
                                    blue_factor,
                                    base_shift,
                                },
                                terrain_matrix,
                            },
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

#[derive(Deserialize)]
struct EncodedFeature {
    key: String,
    tile_key: TileKey,
    layer_id: String,
    id: Option<String>,
    geometry: Geometry,
    #[serde(default)]
    properties: serde_json::Value,
}

fn parse_features(encoded_features: &str) -> Vec<MapViewFeatureData> {
    serde_json::from_str::<Vec<EncodedFeature>>(encoded_features)
        .unwrap_or_default()
        .into_iter()
        .map(|feature| {
            let key = MapViewFeatureKey {
                tile_key: feature.tile_key,
                layer_id: feature.layer_id.clone(),
                id: feature.key,
            };

            MapViewFeatureData {
                key,
                layer_id: feature.layer_id,
                id: feature.id,
                geometry: feature.geometry,
                properties: feature.properties,
            }
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
