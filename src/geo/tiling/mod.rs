use crate::geo::coords::{BoundedMercatorProjection, Projection2D, approx_size_bound};
use backend_model::earth_tiling_service_model::{
    GetTileRequestParams, GibsLayer, Layer, TileSubKey,
};
use bevy::prelude::Reflect;
use glam::{USizeVec2, dvec2};
use itertools::Itertools;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use thiserror::Error;
use utilities::glam_ext::bounding::{AxisAlignedBoundingBox2D, DAabb2};
use utilities::glam_ext::sub_division::{SubDivision2d, SubDivisionKey, TileKey};

#[derive(Clone)]
pub struct TileServer {
    pub file_cache_dir: PathBuf,
    pub tile_resolution_width: usize,

    client: reqwest::Client,
    cache: Arc<tokio::sync::RwLock<HashMap<PathBuf, Option<PathBuf>>>>,
}

#[derive(Debug, Error)]
pub enum TileServerError {
    #[error("HTTP Error: {0:?}")]
    HttpError(#[from] reqwest::Error),
    #[error("IO Error: {0:?}")]
    IoError(#[from] std::io::Error),
    #[error("Image Error: {0:?}")]
    ImageError(#[from] image::ImageError),
    #[error("Reprojection Error")]
    ReprojectionError,
    #[error("Retry Error")]
    RetryError,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Reflect)]
pub enum TileServerDataset {
    GibsLayerModisTerraCorrectedReflectanceTrueColor,
    SenHubSentinel2L2a,
}

impl TileServer {
    pub fn new(tile_resolution_width: usize, cache_dir: impl AsRef<Path>) -> Self {
        Self {
            tile_resolution_width,
            client: reqwest::Client::new(),
            cache: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            file_cache_dir: PathBuf::from(cache_dir.as_ref()),
        }
    }

    fn is_tile_available(
        &self,
        dataset: &TileServerDataset,
        projection: &BoundedMercatorProjection,
        tile_key: &TileKey,
    ) -> bool {
        let gcs_bbox = self.tile_gcs_bbox(projection, tile_key);
        let res = self.tile_resolution(gcs_bbox);

        let meters_per_pixel =
            (approx_size_bound(gcs_bbox) / dvec2(res.x as f64, res.y as f64)).max_element();

        match dataset {
            TileServerDataset::SenHubSentinel2L2a => (5.0..=1400.0).contains(&meters_per_pixel),
            TileServerDataset::GibsLayerModisTerraCorrectedReflectanceTrueColor => {
                meters_per_pixel > 50.0
            }
        }
    }

    fn get_cache_key(
        &self,
        dataset: TileServerDataset,
        projection: &BoundedMercatorProjection,
        tile: &TileKey,
    ) -> PathBuf {
        PathBuf::from_iter([
            hex::encode(format!("{dataset:?}")).as_str(),
            hex::encode(format!("{projection:?}")).as_str(),
            self.tile_resolution_width.to_string().as_str(),
            format!(
                "tile_{}",
                tile.iter()
                    .map(|sub_key| match sub_key {
                        SubDivisionKey::TopLeft => "TL",
                        SubDivisionKey::TopRight => "TR",
                        SubDivisionKey::BottomLeft => "BL",
                        SubDivisionKey::BottomRight => "BR",
                    })
                    .join("_")
            )
            .as_str(),
        ])
    }

    fn get_tile_file_path(
        &self,
        dataset: TileServerDataset,
        projection: &BoundedMercatorProjection,
        tile_key: &TileKey,
    ) -> PathBuf {
        let key = self.get_cache_key(dataset.clone(), projection, tile_key);
        let file_key = key.with_added_extension("jpg");
        self.file_cache_dir.join(&file_key)
    }

    pub fn load_tile_offline_blocking(
        &self,
        dataset: TileServerDataset,
        projection: &BoundedMercatorProjection,
        tile_key: &TileKey,
    ) -> Result<Option<Option<PathBuf>>, TileServerError> {
        if !self.is_tile_available(&dataset, projection, tile_key) {
            return Ok(Some(None));
        }

        let file_path = self.get_tile_file_path(dataset, projection, tile_key);

        if let Some(cached_value) = self.cache.blocking_read().get(&file_path).cloned() {
            return Ok(Some(cached_value));
        }

        if std::fs::exists(&file_path)? {
            self.cache
                .blocking_write()
                .insert(file_path.clone(), Some(file_path.clone()));

            return Ok(Some(Some(file_path)));
        }

        Ok(None)
    }

    pub async fn load_tile_offline(
        &self,
        dataset: TileServerDataset,
        projection: &BoundedMercatorProjection,
        tile_key: &TileKey,
    ) -> Result<Option<Option<PathBuf>>, TileServerError> {
        if !self.is_tile_available(&dataset, projection, tile_key) {
            return Ok(Some(None));
        }

        let file_path = self.get_tile_file_path(dataset, projection, tile_key);

        if let Some(cached_value) = self.cache.read().await.get(&file_path).cloned() {
            return Ok(Some(cached_value));
        }

        if tokio::fs::try_exists(&file_path).await? {
            self.cache
                .write()
                .await
                .insert(file_path.clone(), Some(file_path.clone()));

            return Ok(Some(Some(file_path)));
        }

        Ok(None)
    }

    fn tile_gcs_bbox(&self, projection: &BoundedMercatorProjection, tile_key: &TileKey) -> DAabb2 {
        let sub_div = SubDivision2d {
            area: projection.abs_bounds(),
        };
        let abs_bbox = sub_div.tile_bbox(tile_key);
        DAabb2::new(
            projection.abs_to_gcs(abs_bbox.min()),
            projection.abs_to_gcs(abs_bbox.max()),
        )
    }

    fn tile_resolution(&self, _gcs_bounds: DAabb2) -> USizeVec2 {
        // let gcs_size = Vec2::from(gcs_bbox.1.clone()) - Vec2::from(gcs_bbox.0.clone());
        // USizeVec2::new(self.tile_resolution_width, (self.tile_resolution_width as f32 * gcs_size.y / gcs_size.x).ceil() as usize)
        USizeVec2::new(self.tile_resolution_width, self.tile_resolution_width)
    }

    pub async fn load_tile(
        &self,
        dataset: TileServerDataset,
        projection: &BoundedMercatorProjection,
        tile_key: &TileKey,
    ) -> Result<Option<PathBuf>, TileServerError> {
        if let Some(cached_value) = self
            .load_tile_offline(dataset.clone(), projection, tile_key)
            .await?
        {
            return Ok(cached_value.clone());
        }

        let file_path = self.get_tile_file_path(dataset.clone(), projection, tile_key);

        if tokio::fs::try_exists(&file_path).await? {
            self.cache
                .write()
                .await
                .insert(file_path.clone(), Some(file_path.clone()));
            return Ok(Some(file_path));
        }

        let res = self
            .client
            .get("http://localhost:80/tile")
            .json(&GetTileRequestParams {
                tile_key: backend_model::earth_tiling_service_model::TileKey::from_iter(
                    tile_key.iter().map(|x| match x {
                        SubDivisionKey::BottomLeft => TileSubKey::BottomLeft,
                        SubDivisionKey::BottomRight => TileSubKey::BottomRight,
                        SubDivisionKey::TopLeft => TileSubKey::TopLeft,
                        SubDivisionKey::TopRight => TileSubKey::TopRight,
                    }),
                ),
                projection:
                    backend_model::earth_tiling_service_model::Projection::BoundedMercator {
                        min_lat: projection.lat_min.to_degrees(),
                        max_lat: projection.lat_max.to_degrees(),
                    },
                layer: match dataset {
                    TileServerDataset::GibsLayerModisTerraCorrectedReflectanceTrueColor => {
                        Layer::Gibs(GibsLayer::LayerModisTerraCorrectedReflectanceTrueColor)
                    }
                    TileServerDataset::SenHubSentinel2L2a => Layer::SenHub,
                },
            })
            .send()
            .await?
            .error_for_status()?;

        let img = image::load_from_memory(&res.bytes().await?)?;

        if let Some(file_dir) = file_path.parent() {
            tokio::fs::create_dir_all(file_dir).await?;
        }
        img.save(&file_path)?;

        self.cache
            .write()
            .await
            .insert(file_path.clone(), Some(file_path.clone()));

        Ok(Some(file_path))
    }
}
