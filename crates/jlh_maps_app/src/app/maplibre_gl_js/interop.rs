use crate::app::instance_management::commands::enqueue_instance_command;
use crate::app::maplibre_gl_js::integration::{
    MaplibreMapIntegration, NEXT_INTEGRATION_ID, find_map_integration, with_map_data_mut,
};
use crate::app::maplibre_gl_js::types::{
    CanonicalTileId, MaplibreMapViewData, MaplibreTerrainTileData, SourceLayerFeature,
};
use crate::utils::dem_data::DEMData;
use crate::utils::terrain::TerrainData;
use anyhow::anyhow;
use bevy::math::DMat4;
use bevy::prelude::{Name, default};
use geojson::Geometry;
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use tracing::error;
use wasm_bindgen::prelude::wasm_bindgen;

#[derive(Deserialize)]
struct EncodedFeature {
    source_id: String,
    source_layer_id: String,
    tile_key: CanonicalTileId,
    feature_id: String,
    geometry: Geometry,
    #[serde(default)]
    properties: HashMap<String, serde_json::Value>,
}

struct ParsedFeature {
    source_id: String,
    source_layer_id: String,
    tile_id: CanonicalTileId,
    feature: SourceLayerFeature,
}

#[derive(Deserialize)]
struct EncodedFeatureKey {
    source_id: String,
    source_layer_id: String,
    tile_key: CanonicalTileId,
    feature_id: String,
}

fn parse_features(encoded_features: &str) -> Vec<ParsedFeature> {
    serde_json::from_str::<Vec<EncodedFeature>>(encoded_features)
        .inspect_err(|err| error!("Failed to parse features: {}", err))
        .unwrap_or_default()
        .into_iter()
        .map(|feature| ParsedFeature {
            source_id: feature.source_id,
            source_layer_id: feature.source_layer_id,
            tile_id: feature.tile_key,
            feature: SourceLayerFeature {
                tile_id: feature.tile_key,
                id: feature.feature_id,
                geometry: feature.geometry,
                properties: feature.properties,
            },
        })
        .collect()
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
    main_matrix_json: String,
) -> Result<(), String> {
    let main_matrix = serde_json::from_str(&main_matrix_json).unwrap_or_default();

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
pub fn update_features(
    instance_id: String,
    integration_id: u32,
    encoded_features: String,
) -> Result<(), String> {
    let features = parse_features(&encoded_features);

    enqueue_instance_command(&instance_id, move |world| {
        with_map_data_mut(world, integration_id, |map_data| {
            for feature in features {
                map_data.features.insert(
                    feature.source_id,
                    feature.source_layer_id,
                    feature.tile_id,
                    feature.feature,
                );
            }
        });
    })
    .map_err(|err| err.to_string())
}

#[wasm_bindgen]
pub fn remove_features(
    instance_id: String,
    integration_id: u32,
    encoded_feature_keys: String,
) -> Result<(), String> {
    let feature_keys = serde_json::from_str::<Vec<EncodedFeatureKey>>(&encoded_feature_keys)
        .inspect_err(|err| error!("Failed to parse removed feature keys: {}", err))
        .unwrap_or_default();

    enqueue_instance_command(&instance_id, move |world| {
        with_map_data_mut(world, integration_id, |map_data| {
            for feature_key in feature_keys {
                map_data.features.remove(
                    &feature_key.source_id,
                    &feature_key.source_layer_id,
                    &feature_key.tile_key,
                    &feature_key.feature_id,
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
