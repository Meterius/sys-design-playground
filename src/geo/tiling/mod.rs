use std::path::PathBuf;
use bevy::math::Vec2;
use bevy::prelude::info;
use image::codecs::jpeg::JpegEncoder;
use itertools::Itertools;
use thiserror::Error;
use crate::geo::coords::{BoundedMercatorProjection, LonLatVec2};
use crate::geo::sub_division::{SubDivision2d, SubDivisionKey, TileKey};
use crate::geo::tiling::gibs::{fetch_epsg4326_image, GibsEpsg4326Params, LAYER_MODIS_TERRA_CORRECTED_REFLECTANCE_TRUE_COLOR};

pub mod gibs;

#[derive(Clone)]
pub struct TileServer {
    pub client: reqwest::Client,
    pub cache_dir: PathBuf,
}

#[derive(Debug, Error)]
pub enum TileServerError{
    #[error("HTTP Error: {0:?}")]
    HttpError(#[from] reqwest::Error),
    #[error("IO Error: {0:?}")]
    IoError(#[from] std::io::Error),
    #[error("Image Error: {0:?}")]
    ImageError(#[from] image::ImageError),
}

impl TileServer {
    fn get_cache_key(projection: &BoundedMercatorProjection, tile: &TileKey) -> PathBuf {
        PathBuf::from_iter([
            LAYER_MODIS_TERRA_CORRECTED_REFLECTANCE_TRUE_COLOR,
            hex::encode(format!("{projection:?}")).as_str(),
            format!("tile_{}", tile.iter().map(|sub_key| match sub_key {
                SubDivisionKey::TopLeft => "TL",
                SubDivisionKey::TopRight => "TR",
                SubDivisionKey::BottomLeft => "BL",
                SubDivisionKey::BottomRight => "BR",
            }).join("_")).as_str(),
        ].into_iter())
    }

    pub async fn load_tile(
        &self,
        projection: &BoundedMercatorProjection,
        tile_key: &TileKey,
    ) -> Result<PathBuf, TileServerError> {
        let key = Self::get_cache_key(projection, &tile_key);
        let file_key = key.with_added_extension("jpg");
        let file_path = self.cache_dir.join(&file_key);

        if tokio::fs::try_exists(&file_path).await? {
            return Ok(file_path);
        }

        let sub_div = SubDivision2d::from_corners(-Vec2::ONE/2.0, Vec2::ONE/2.0);
        let rel_bbox = sub_div.tile_bbox(tile_key);
        let gcs_bbox= (
            LonLatVec2::from(projection.abs_to_gcs(&(projection.abs_pos() + rel_bbox.0 * projection.abs_size()))),
            LonLatVec2::from(projection.abs_to_gcs(&(projection.abs_pos() + rel_bbox.1 * projection.abs_size()))),
        );

        info!("{tile_key:?} {rel_bbox:?} {gcs_bbox:?}");

       let img = fetch_epsg4326_image(&self.client, GibsEpsg4326Params {
            layers: LAYER_MODIS_TERRA_CORRECTED_REFLECTANCE_TRUE_COLOR.to_owned(),
            bbox: (gcs_bbox.0.x, gcs_bbox.0.y, gcs_bbox.1.x, gcs_bbox.1.y),
            size: (256, 256),
        }).await?;

        if let Some(file_dir) = file_path.parent() {
            tokio::fs::create_dir_all(file_dir).await?;
        }
        img.save(&file_path)?;

        Ok(file_path)
    }
}

