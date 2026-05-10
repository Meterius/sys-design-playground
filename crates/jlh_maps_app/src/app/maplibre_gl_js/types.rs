use bevy::prelude::Reflect;
use geojson::Geometry;
use serde::Deserialize;
use std::collections::{HashMap, HashSet};

use crate::utils::terrain::TerrainData;

#[derive(Default)]
#[allow(dead_code)]
pub struct MaplibreMapViewData {
    pub width: f64,
    pub height: f64,
    pub zoom: f64,
    pub pitch: f64,
    pub bearing: f64,
    pub center_lng: f64,
    pub center_lat: f64,
    pub main_matrix: Vec<f64>,
}

#[derive(Default)]
pub struct MaplibreTerrainData {
    pub active_tile_ids: HashSet<CanonicalTileId>,
    pub tiles: HashMap<CanonicalTileId, MaplibreTerrainTileData>,
}

#[derive(Clone, Debug)]
pub struct MaplibreTerrainTileData {
    pub hash: String,
    pub terrain_data: TerrainData,
}

#[derive(Default)]
pub struct MaplibreFeatureData {
    pub features: HashMap<String, SourceLayerFeature>,
}

#[derive(Clone, Debug)]
pub struct SourceLayerFeature {
    pub tile_id: CanonicalTileId,
    pub id: String,
    pub source_layer_id: String,
    pub geometry: Geometry,
    pub properties: serde_json::Value,
}

#[derive(Reflect, Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq)]
pub struct CanonicalTileId {
    pub z: u32,
    pub x: u32,
    pub y: u32,
}
