use crate::geo::coords::{BoundedMercatorProjection, LonLatVec2, RadLonLatVec2};
use crate::geo::sub_division::{SubDivision2d, SubDivisionKey, TileKey};
use crate::geo::tiling::gibs::{
    GibsEpsg4326Params, LAYER_MODIS_TERRA_CORRECTED_REFLECTANCE_TRUE_COLOR, fetch_epsg4326_image,
};
use bevy::math::Vec2;
use itertools::Itertools;
use std::path::PathBuf;
use thiserror::Error;
use crate::geo::tiling::TileServerError::ReprojectionError;

pub mod gibs;

#[derive(Clone)]
pub struct TileServer {
    pub client: reqwest::Client,
    pub cache_dir: PathBuf,
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
}

impl TileServer {
    fn get_cache_key(projection: &BoundedMercatorProjection, tile: &TileKey) -> PathBuf {
        PathBuf::from_iter(
            [
                LAYER_MODIS_TERRA_CORRECTED_REFLECTANCE_TRUE_COLOR,
                hex::encode(format!("{projection:?}")).as_str(),
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
            ]
            .into_iter(),
        )
    }

    fn reprojected(
        image: &image::RgbImage,
        projection: &BoundedMercatorProjection,
        gcs_bbox: (RadLonLatVec2, RadLonLatVec2),
    ) -> Result<image::RgbImage, TileServerError> {
        let mut out = image::RgbImage::new(image.width(), image.height());

        let abs_min = projection.gcs_to_abs(&gcs_bbox.0);
        let abs_size = projection.gcs_to_abs(&gcs_bbox.1) - abs_min;

        let gcs_min = Vec2::from(gcs_bbox.0.clone());
        let gcs_size = Vec2::from(gcs_bbox.1.clone()) - Vec2::from(gcs_bbox.0.clone());

        let image_size = Vec2::new(image.width() as f32, image.height() as f32);

        for (x, y, pixel) in out.enumerate_pixels_mut() {
            let abs_pos = abs_min + abs_size * (Vec2::new(0.0, 1.0) + Vec2::new(x as f32 + 0.5, -(y as f32 + 0.5)) / image_size);
            let gcs_pos = projection.abs_to_gcs(&abs_pos);
            let img_pos_rel = (
                Vec2::new(0.0, 1.0)
                    + Vec2::new(1.0, -1.0) * (Vec2::from(gcs_pos.clone()) - gcs_min) / gcs_size
            ).clamp(Vec2::ZERO, Vec2::ONE);
            *pixel = image::imageops::sample_bilinear(image, img_pos_rel.x, img_pos_rel.y)
                .ok_or_else(|| TileServerError::ReprojectionError)?;
        }

        Ok(out)
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

        let sub_div = SubDivision2d::from_corners(-Vec2::ONE / 2.0, Vec2::ONE / 2.0);
        let rel_bbox = sub_div.tile_bbox(tile_key);
        let rel_size = rel_bbox.1 - rel_bbox.0;
        let rad_gcs_bbox = (
            projection.abs_to_gcs(&(projection.abs_pos() + rel_bbox.0 * projection.abs_size())),
            projection.abs_to_gcs(&(projection.abs_pos() + rel_bbox.1 * projection.abs_size())),
        );

        let gcs_bbox = (
            LonLatVec2::from(rad_gcs_bbox.0.clone()),
            LonLatVec2::from(rad_gcs_bbox.1.clone()),
        );

        let img = fetch_epsg4326_image(
            &self.client,
            GibsEpsg4326Params {
                layers: LAYER_MODIS_TERRA_CORRECTED_REFLECTANCE_TRUE_COLOR.to_owned(),
                bbox: (gcs_bbox.0.x, gcs_bbox.0.y, gcs_bbox.1.x, gcs_bbox.1.y),
                size: (256, (256.0 * rel_size.y / rel_size.x).ceil() as usize),
            },
        )
        .await?;

        let img = Self::reprojected(&img, projection, rad_gcs_bbox)?;

        if let Some(file_dir) = file_path.parent() {
            tokio::fs::create_dir_all(file_dir).await?;
        }
        img.save(&file_path)?;

        Ok(file_path)
    }
}
