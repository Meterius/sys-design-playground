use itertools::Itertools;
use crate::app::instance_management::commands::enqueue_instance_command;
use crate::app::maplibre_gl_js::integration::{
    MaplibreMapIntegration, NEXT_INTEGRATION_ID, find_map_integration, with_map_data_mut,
};
use crate::app::maplibre_gl_js::types::{
    CanonicalTileId, MaplibreMapViewData, MaplibreTerrainTileData, SourceLayerFeature,
};
use crate::app::maplibre_gl_js::utils::dem_data::DEMData;
use crate::app::maplibre_gl_js::utils::terrain::TerrainData;
use anyhow::anyhow;
use bevy::math::DMat4;
use bevy::prelude::{Name, default};
use geojson::Geometry;
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use tracing::info;
use wasm_bindgen::prelude::{JsValue, wasm_bindgen};

#[derive(Deserialize)]
struct EncodedTileFeature {
    feature_id: u64,
    geometry: Geometry,
    #[serde(default)]
    properties: HashMap<String, serde_json::Value>,
}

#[derive(Deserialize)]
struct EncodedFeatureTile {
    source_id: String,
    source_layer_id: String,
    tile_key: CanonicalTileId,
    #[serde(default)]
    features: Vec<EncodedTileFeature>,
}

struct ParsedFeatureTile {
    source_id: String,
    source_layer_id: String,
    tile_id: CanonicalTileId,
    features: Vec<SourceLayerFeature>,
}

#[derive(Deserialize)]
struct EncodedFeatureTileRemoval {
    source_id: String,
    source_layer_id: String,
    tile_key: CanonicalTileId,
    #[serde(default)]
    feature_ids: Vec<u64>,
}

fn parse_feature_tiles(feature_tiles: JsValue) -> Result<Vec<ParsedFeatureTile>, String> {
    Ok(
        serde_wasm_bindgen::from_value::<Vec<EncodedFeatureTile>>(feature_tiles)
            .map_err(|err| format!("Failed to parse feature tiles: {err}"))?
            .into_iter()
            .map(|tile| {
                let tile_id = tile.tile_key;
                ParsedFeatureTile {
                    source_id: tile.source_id,
                    source_layer_id: tile.source_layer_id,
                    tile_id,
                    features: tile
                        .features
                        .into_iter()
                        .map(|feature| SourceLayerFeature {
                            tile_id,
                            id: feature.feature_id,
                            geometry: feature.geometry,
                            properties: feature.properties,
                        })
                        .collect(),
                }
            })
            .collect(),
    )
}

fn parse_feature_tile_removals(
    feature_tiles: JsValue,
) -> Result<Vec<EncodedFeatureTileRemoval>, String> {
    serde_wasm_bindgen::from_value(feature_tiles)
        .map_err(|err| format!("Failed to parse removed feature tiles: {err}"))
}

fn parse_tile_key(tile: &str) -> anyhow::Result<CanonicalTileId> {
    let mut parts = tile.split('/');
    let z = parts
        .next()
        .ok_or(anyhow!("Missing z coordinate in tile key"))?
        .parse()?;
    let x = parts
        .next()
        .ok_or(anyhow!("Missing x coordinate in tile key"))?
        .parse()?;
    let y = parts
        .next()
        .ok_or(anyhow!("Missing y coordinate in tile key"))?
        .parse()?;

    Ok(CanonicalTileId { z, x, y })
}

fn parse_terrain_matrix(encoded: &str) -> anyhow::Result<DMat4> {
    let terrain_matrix = serde_json::from_str::<Vec<f64>>(encoded)?;
    terrain_matrix
        .as_slice()
        .try_into()
        .map(DMat4::from_cols_array)
        .map_err(|_| anyhow!("Invalid terrain matrix format"))
}

#[wasm_bindgen]
pub fn create_map_integration(instance_id: String) -> Result<u32, String> {
    let id = NEXT_INTEGRATION_ID.with(|next| {
        let id = next.get();
        next.set(id.saturating_add(1).max(1));
        id
    });

    enqueue_instance_command(&instance_id, move |world| {
        world.spawn((
            MaplibreMapIntegration { id, ..default() },
            Name::new(format!("MapLibre map integration {id}")),
        ));
    })
    .map_err(|err| err.to_string())?;

    Ok(id)
}

#[wasm_bindgen]
pub fn remove_map_integration(instance_id: String, integration_id: u32) -> Result<(), String> {
    enqueue_instance_command(&instance_id, move |world| {
        if let Some(entity) = find_map_integration(world, integration_id) {
            world.despawn(entity);
        }
    })
    .map_err(|err| err.to_string())
}

#[allow(clippy::too_many_arguments)]
#[wasm_bindgen]
pub fn sync_view(
    instance_id: String,
    integration_id: u32,
    width: f64,
    height: f64,
    zoom: f64,
    pitch: f64,
    bearing: f64,
    center_lng: f64,
    center_lat: f64,
    main_matrix: Vec<f64>,
) -> Result<(), String> {
    if main_matrix.len() != 16 {
        return Err(format!(
            "Expected 16 main matrix values, got {}",
            main_matrix.len()
        ));
    }

    let view = MaplibreMapViewData {
        width,
        height,
        zoom,
        pitch,
        bearing,
        center_lng,
        center_lat,
        main_matrix,
    };

    enqueue_instance_command(&instance_id, move |world| {
        with_map_data_mut(world, integration_id, |map_data| {
            map_data.view = view;
        });
    })
    .map_err(|err| err.to_string())
}

#[wasm_bindgen]
#[allow(clippy::too_many_arguments)]
pub fn update_terrain_tile_data(
    instance_id: String,
    integration_id: u32,
    tile_key: String,
    hash: String,
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
) -> Result<(), String> {
    let tile_key = parse_tile_key(&tile_key).map_err(|err| err.to_string())?;
    let terrain_matrix =
        parse_terrain_matrix(&terrain_matrix_json).map_err(|err| err.to_string())?;

    let tile_data = MaplibreTerrainTileData {
        hash,
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
    };

    enqueue_instance_command(&instance_id, move |world| {
        with_map_data_mut(world, integration_id, |map_data| {
            map_data.terrain.tiles.insert(tile_key, tile_data);
        });
    })
    .map_err(|err| err.to_string())
}

#[wasm_bindgen]
pub fn remove_terrain_tile_data(
    instance_id: String,
    integration_id: u32,
    tile_key: String,
) -> Result<(), String> {
    let tile_key = parse_tile_key(&tile_key).map_err(|err| err.to_string())?;

    enqueue_instance_command(&instance_id, move |world| {
        with_map_data_mut(world, integration_id, |map_data| {
            map_data.terrain.tiles.remove(&tile_key);
        });
    })
    .map_err(|err| err.to_string())
}

#[wasm_bindgen]
pub fn update_feature_tiles(
    instance_id: String,
    integration_id: u32,
    feature_tiles: JsValue,
) -> Result<(), String> {
    let feature_tiles = parse_feature_tiles(feature_tiles)?;

    info!("updated {}", feature_tiles.iter().map(|f| format!("{:?} {}", f.tile_id, f.features.iter().count())).join(", "));

    enqueue_instance_command(&instance_id, move |world| {
        with_map_data_mut(world, integration_id, |map_data| {
            for feature_tile in feature_tiles {
                map_data.features.insert_tile_features(
                    feature_tile.source_id,
                    feature_tile.source_layer_id,
                    feature_tile.tile_id,
                    feature_tile.features,
                );
            }
        });
    })
    .map_err(|err| err.to_string())
}

#[wasm_bindgen]
pub fn remove_feature_tiles(
    instance_id: String,
    integration_id: u32,
    feature_tiles: JsValue,
) -> Result<(), String> {
    let feature_tiles = parse_feature_tile_removals(feature_tiles)?;

    info!("removed {}", feature_tiles.iter().map(|f| format!("{:?} {}", f.tile_key, f.feature_ids.len())).join(", "));

    enqueue_instance_command(&instance_id, move |world| {
        with_map_data_mut(world, integration_id, |map_data| {
            for feature_tile in feature_tiles {
                map_data.features.remove_tile_features(
                    &feature_tile.source_id,
                    &feature_tile.source_layer_id,
                    &feature_tile.tile_key,
                    &feature_tile.feature_ids,
                );
            }
        });
    })
    .map_err(|err| err.to_string())
}

#[wasm_bindgen]
pub fn sync_terrain_active_tile_ids(
    instance_id: String,
    integration_id: u32,
    active_tile_ids: Vec<String>,
) -> Result<(), String> {
    let active_tile_ids = active_tile_ids
        .into_iter()
        .map(|v| parse_tile_key(&v))
        .collect::<Result<HashSet<_>, _>>()
        .map_err(|err| err.to_string())?;

    enqueue_instance_command(&instance_id, move |world| {
        with_map_data_mut(world, integration_id, |map_data| {
            map_data.terrain.active_tile_ids = active_tile_ids;
        });
    })
    .map_err(|err| err.to_string())
}
