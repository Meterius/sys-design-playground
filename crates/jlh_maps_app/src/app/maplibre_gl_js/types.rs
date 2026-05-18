use bevy::prelude::Reflect;
use geojson::Geometry;
use serde::Deserialize;
use std::collections::{HashMap, HashSet};

use crate::app::maplibre_gl_js::utils::terrain::TerrainData;

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
    pub sources: HashMap<String, MaplibreFeatureSourceData>,
}

impl MaplibreFeatureData {
    pub fn insert_tile_features(
        &mut self,
        source_id: String,
        source_layer_id: String,
        tile_id: CanonicalTileId,
        features: Vec<SourceLayerFeature>,
    ) {
        if features.is_empty() {
            return;
        }

        let tile_features = self
            .sources
            .entry(source_id)
            .or_default()
            .source_layers
            .entry(source_layer_id)
            .or_default()
            .tiles
            .entry(tile_id)
            .or_default();

        for feature in features {
            tile_features.insert(feature.id, feature);
        }
    }

    pub fn insert(
        &mut self,
        source_id: String,
        source_layer_id: String,
        tile_id: CanonicalTileId,
        feature: SourceLayerFeature,
    ) {
        self.sources
            .entry(source_id)
            .or_default()
            .source_layers
            .entry(source_layer_id)
            .or_default()
            .tiles
            .entry(tile_id)
            .or_default()
            .insert(feature.id, feature);
    }

    pub fn remove_tile_features(
        &mut self,
        source_id: &str,
        source_layer_id: &str,
        tile_id: &CanonicalTileId,
        feature_ids: &[u64],
    ) {
        let Some(source) = self.sources.get_mut(source_id) else {
            return;
        };
        let remove_source = source.remove_tile_features(source_layer_id, tile_id, feature_ids);
        if remove_source {
            self.sources.remove(source_id);
        }
    }

    pub fn remove(
        &mut self,
        source_id: &str,
        source_layer_id: &str,
        tile_id: &CanonicalTileId,
        feature_id: u64,
    ) {
        let Some(source) = self.sources.get_mut(source_id) else {
            return;
        };
        let remove_source = source.remove(source_layer_id, tile_id, feature_id);
        if remove_source {
            self.sources.remove(source_id);
        }
    }
}

#[derive(Default)]
pub struct MaplibreFeatureSourceData {
    pub source_layers: HashMap<String, MaplibreFeatureSourceLayerData>,
}

impl MaplibreFeatureSourceData {
    fn remove_tile_features(
        &mut self,
        source_layer_id: &str,
        tile_id: &CanonicalTileId,
        feature_ids: &[u64],
    ) -> bool {
        let Some(source_layer) = self.source_layers.get_mut(source_layer_id) else {
            return self.source_layers.is_empty();
        };
        let remove_source_layer = source_layer.remove_tile_features(tile_id, feature_ids);
        if remove_source_layer {
            self.source_layers.remove(source_layer_id);
        }

        self.source_layers.is_empty()
    }

    fn remove(
        &mut self,
        source_layer_id: &str,
        tile_id: &CanonicalTileId,
        feature_id: u64,
    ) -> bool {
        let Some(source_layer) = self.source_layers.get_mut(source_layer_id) else {
            return self.source_layers.is_empty();
        };
        let remove_source_layer = source_layer.remove(tile_id, feature_id);
        if remove_source_layer {
            self.source_layers.remove(source_layer_id);
        }

        self.source_layers.is_empty()
    }
}

#[derive(Default)]
pub struct MaplibreFeatureSourceLayerData {
    pub tiles: HashMap<CanonicalTileId, HashMap<u64, SourceLayerFeature>>,
}

impl MaplibreFeatureSourceLayerData {
    fn remove_tile_features(&mut self, tile_id: &CanonicalTileId, feature_ids: &[u64]) -> bool {
        if feature_ids.is_empty() {
            self.tiles.remove(tile_id);
            return self.tiles.is_empty();
        }

        let Some(features) = self.tiles.get_mut(tile_id) else {
            return self.tiles.is_empty();
        };

        for feature_id in feature_ids {
            features.remove(feature_id);
        }
        if features.is_empty() {
            self.tiles.remove(tile_id);
        }

        self.tiles.is_empty()
    }

    fn remove(&mut self, tile_id: &CanonicalTileId, feature_id: u64) -> bool {
        let Some(features) = self.tiles.get_mut(tile_id) else {
            return self.tiles.is_empty();
        };

        features.remove(&feature_id);
        if features.is_empty() {
            self.tiles.remove(tile_id);
        }

        self.tiles.is_empty()
    }
}

#[derive(Clone, Debug)]
pub struct SourceLayerFeature {
    pub tile_id: CanonicalTileId,
    pub id: u64,
    pub geometry: Geometry,
    pub properties: HashMap<String, serde_json::Value>,
}

#[derive(Reflect, Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq)]
pub struct CanonicalTileId {
    pub z: u32,
    pub x: u32,
    pub y: u32,
}
