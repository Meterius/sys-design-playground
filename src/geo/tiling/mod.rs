use crate::geo::coords::{BoundedMercatorProjection, Projection2D, approx_size_bound};
use crate::geo::tiling::image_sources::{
    Epsg4326TileParams, GibsEpsg4326Params, LAYER_MODIS_TERRA_CORRECTED_REFLECTANCE_TRUE_COLOR,
    fetch_epsg4326_gibs_image, fetch_epsg4326_sen_hub_image, fetch_sen_hub_bearer_token,
};
use bevy::prelude::Reflect;
use glam::{DVec2, USizeVec2, dvec2};
use itertools::Itertools;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use thiserror::Error;
use utilities::glam_ext::bounding::{AxisAlignedBoundingBox2D, DAabb2};
use utilities::glam_ext::sub_division::{SubDivision2d, SubDivisionKey, TileKey};

pub mod image_sources;

#[derive(Clone)]
pub struct TileServer {
    pub file_cache_dir: PathBuf,
    pub tile_resolution_width: usize,

    client: reqwest::Client,
    cache: Arc<tokio::sync::RwLock<HashMap<PathBuf, Option<PathBuf>>>>,
    sen_hub_bearer_token: Arc<tokio::sync::Mutex<Option<Result<String, TileServerError>>>>,
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
            sen_hub_bearer_token: Arc::new(tokio::sync::Mutex::new(None)),
        }
    }

    #[allow(clippy::useless_asref)]
    async fn get_sen_hub_bearer_token(&self) -> Result<String, TileServerError> {
        {
            let token_mtx = self.sen_hub_bearer_token.lock().await;
            if let Some(token) = &*token_mtx {
                return token
                    .as_ref()
                    .map(Clone::clone)
                    .map_err(|_| TileServerError::RetryError);
            }
        }

        let mut token_mtx = self.sen_hub_bearer_token.lock().await;

        if let Some(token) = &*token_mtx {
            return token
                .as_ref()
                .map(Clone::clone)
                .map_err(|_| TileServerError::RetryError);
        }

        let token: Result<String, TileServerError> = fetch_sen_hub_bearer_token(&self.client)
            .await
            .map_err(Into::into);

        *token_mtx = Some(
            token
                .as_ref()
                .map(Clone::clone)
                .map_err(|_| TileServerError::RetryError),
        );

        token
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
                meters_per_pixel > 1400.0
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

    fn reprojected(
        image: &image::RgbImage,
        projection: &impl Projection2D,
        gcs_bounds: DAabb2,
    ) -> Result<image::RgbImage, TileServerError> {
        let mut out = image::RgbImage::new(image.width(), image.height());

        let abs_bounds = DAabb2::new(
            projection.gcs_to_abs(gcs_bounds.min()),
            projection.gcs_to_abs(gcs_bounds.max()),
        );

        let image_size = DVec2::new(image.width() as f64, image.height() as f64);

        for (x, y, pixel) in out.enumerate_pixels_mut() {
            let rel = dvec2(0.0, 1.0) + dvec2(x as f64 + 0.5, -(y as f64 + 0.5)) / image_size;
            debug_assert!((0.0..=1.0).contains(&rel.x) && (0.0..=1.0).contains(&rel.y));
            let abs_pos = abs_bounds.min() + abs_bounds.size() * rel;
            let gcs_pos = projection.abs_to_gcs(abs_pos);

            let img_pos_rel = dvec2(0.0, 1.0)
                + dvec2(1.0, -1.0) * (gcs_pos - gcs_bounds.min()) / gcs_bounds.size();
            debug_assert!(
                (0.0..=1.0).contains(&img_pos_rel.x) && (0.0..=1.0).contains(&img_pos_rel.y)
            );

            *pixel =
                image::imageops::sample_bilinear(image, img_pos_rel.x as f32, img_pos_rel.y as f32)
                    .ok_or_else(|| TileServerError::ReprojectionError)?;
        }

        Ok(out)
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

        let rad_gcs_bbox = self.tile_gcs_bbox(projection, tile_key);
        let [res_width, res_height] = self.tile_resolution(rad_gcs_bbox).to_array();

        let tile_params = Epsg4326TileParams {
            gcs_bounds: DAabb2::new(
                rad_gcs_bbox.min().map(|a| a.to_degrees()),
                rad_gcs_bbox.max().map(|a| a.to_degrees()),
            ),
            resolution: (res_width, res_height),
        };

        let img = match &dataset {
            TileServerDataset::GibsLayerModisTerraCorrectedReflectanceTrueColor => {
                fetch_epsg4326_gibs_image(
                    &self.client,
                    GibsEpsg4326Params {
                        layers: LAYER_MODIS_TERRA_CORRECTED_REFLECTANCE_TRUE_COLOR.to_owned(),
                        tile_params,
                    },
                )
                .await?
            }
            TileServerDataset::SenHubSentinel2L2a => {
                fetch_epsg4326_sen_hub_image(
                    &self.client,
                    self.get_sen_hub_bearer_token().await?,
                    tile_params,
                )
                .await?
            }
        };

        let img = Self::reprojected(&img, projection, rad_gcs_bbox)?;

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
