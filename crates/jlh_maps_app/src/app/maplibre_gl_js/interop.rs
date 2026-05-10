use crate::app::maplibre_gl_js::integration;
use crate::app::maplibre_gl_js::integration::{MaplibreIntegrationCommand, NEXT_INTEGRATION_ID};
use crate::app::maplibre_gl_js::types::{
    CanonicalTileId, MaplibreMapViewData, MaplibreTerrainTileData, SourceLayerFeature,
};
use crate::utils::dem_data::DEMData;
use crate::utils::terrain::TerrainData;
use bevy::math::DMat4;
use bevy::prelude;
use geojson::Geometry;
use serde::Deserialize;
use tracing::error;
use wasm_bindgen::prelude::wasm_bindgen;

#[derive(Deserialize)]
struct EncodedFeature {
    key: String,
    tile_key: CanonicalTileId,
    layer_id: String,
    geometry: Geometry,
    #[serde(default)]
    properties: serde_json::Value,
}

fn parse_features(encoded_features: &str) -> Vec<SourceLayerFeature> {
    serde_json::from_str::<Vec<EncodedFeature>>(encoded_features)
        .inspect_err(|err| error!("Failed to parse features: {}", err))
        .unwrap_or_default()
        .into_iter()
        .map(|feature| SourceLayerFeature {
            tile_id: feature.tile_key,
            id: feature.key,
            source_layer_id: feature.layer_id,
            geometry: feature.geometry,
            properties: feature.properties,
        })
        .collect()
}

fn parse_tile_key(tile: &str) -> Option<CanonicalTileId> {
    let mut parts = tile.split('/');
    let z = parts.next()?.parse().ok()?;
    let x = parts.next()?.parse().ok()?;
    let y = parts.next()?.parse().ok()?;

    Some(CanonicalTileId { z, x, y })
}

fn parse_terrain_matrix(encoded: &str) -> DMat4 {
    let terrain_matrix = serde_json::from_str::<Vec<f64>>(encoded).unwrap_or_default();
    terrain_matrix
        .as_slice()
        .try_into()
        .map(DMat4::from_cols_array)
        .unwrap_or(DMat4::IDENTITY)
}
#[wasm_bindgen]
pub fn create_map_integration(instance_id: String) -> prelude::Result<u32, String> {
    let id = NEXT_INTEGRATION_ID.with(|next| {
        let id = next.get();
        next.set(id.saturating_add(1).max(1));
        id
    });

    integration::enqueue_command(MaplibreIntegrationCommand::CreateMapIntegration {
        instance_id,
        integration_id: id,
    })?;

    Ok(id)
}

#[wasm_bindgen]
pub fn remove_map_integration(
    instance_id: String,
    integration_id: u32,
) -> prelude::Result<(), String> {
    integration::enqueue_command(MaplibreIntegrationCommand::RemoveMapIntegration {
        instance_id,
        integration_id,
    })
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
) -> prelude::Result<(), String> {
    let main_matrix = serde_json::from_str(&main_matrix_json).unwrap_or_default();

    integration::enqueue_command(MaplibreIntegrationCommand::SyncView {
        instance_id,
        integration_id,
        view: MaplibreMapViewData {
            width,
            height,
            zoom,
            pitch,
            bearing,
            center_lng,
            center_lat,
            main_matrix,
        },
    })
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
) -> prelude::Result<(), String> {
    let Some(tile_key) = parse_tile_key(&tile_key) else {
        return Err("Invalid tile key format".to_string());
    };
    let terrain_matrix = parse_terrain_matrix(&terrain_matrix_json);

    integration::enqueue_command(MaplibreIntegrationCommand::UpdateTerrainTileData {
        instance_id,
        integration_id,
        tile_key,
        tile_data: MaplibreTerrainTileData {
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
        },
    })
}

#[wasm_bindgen]
pub fn remove_terrain_tile_data(
    instance_id: String,
    integration_id: u32,
    tile_key: String,
) -> prelude::Result<(), String> {
    let Some(tile_key) = parse_tile_key(&tile_key) else {
        return Err("Invalid tile key format".to_string());
    };

    integration::enqueue_command(MaplibreIntegrationCommand::RemoveTerrainTileData {
        instance_id,
        integration_id,
        tile_key,
    })
}

#[wasm_bindgen]
pub fn update_features(
    instance_id: String,
    integration_id: u32,
    encoded_features: String,
) -> prelude::Result<(), String> {
    let features = parse_features(&encoded_features);

    integration::enqueue_command(MaplibreIntegrationCommand::UpdateFeatures {
        instance_id,
        integration_id,
        features,
    })
}

#[wasm_bindgen]
pub fn remove_features(
    instance_id: String,
    integration_id: u32,
    encoded_feature_keys: String,
) -> prelude::Result<(), String> {
    let feature_keys = serde_json::from_str::<Vec<String>>(&encoded_feature_keys)
        .inspect_err(|err| error!("Failed to parse removed feature keys: {}", err))
        .unwrap_or_default();

    integration::enqueue_command(MaplibreIntegrationCommand::RemoveFeatures {
        instance_id,
        integration_id,
        feature_keys,
    })
}
