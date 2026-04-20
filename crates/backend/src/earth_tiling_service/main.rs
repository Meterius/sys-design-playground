use actix_files as fs;
use actix_web::error::{ErrorInternalServerError, ErrorNotFound};
use actix_web::{App, HttpServer, get, web};
use backend_model::earth_tiling_service_model::{GetTileRequest, Layer, Projection, TileSubKey};
use cached::proc_macro::once;
use glam::{DVec2, dvec2};
use image::{RgbImage, RgbaImage};
use itertools::Itertools;
use std::path::PathBuf;
use std::sync::Arc;
use thiserror::Error;
use tracing::info;
use utilities::glam_ext::bounding::{AxisAlignedBoundingBox2D, DAabb2};
use utilities::glam_ext::geo::{BoundedMercatorProjection, Projection2D};
use utilities::glam_ext::sub_division::{SubDivision2d, SubDivisionKey};

use super::image_sources::{
    Epsg4326TileParams, GibsEpsg4326Params, LayeredDistributedMappedImage,
    fetch_epsg4326_gibs_image, fetch_epsg4326_local_image, fetch_epsg4326_sen_hub_image,
    fetch_sen_hub_bearer_token,
};

const TILE_DIR: &str = "cache/tiles/";

#[derive(Debug, Error)]
enum TileGenError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("Image error: {0}")]
    Image(#[from] image::ImageError),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Reprojection error")]
    Reprojection,
    #[error("Retry failure")]
    RetryFailure,
    #[error("Error: {0}")]
    Other(#[from] anyhow::Error),
}

struct AppState {
    client: reqwest::Client,
    local_layer: LayeredDistributedMappedImage,
}

#[once(time = 300, sync_writes = true)]
async fn get_sen_hub_bearer_token_cached(
    client: &reqwest::Client,
) -> Result<String, Arc<Box<reqwest::Error>>> {
    fetch_sen_hub_bearer_token(client)
        .await
        .map_err(|err| Arc::new(Box::new(err)))
}

fn tile_sub_dir(params: &GetTileRequest) -> PathBuf {
    PathBuf::from_iter([
        format!("{:?}", params.layer).as_str(),
        hex::encode(format!("{:?}", params.projection)).as_str(),
        format!(
            "tile_{}",
            params
                .tile_key
                .iter()
                .map(|sub_key| match sub_key {
                    TileSubKey::TopLeft => "TL",
                    TileSubKey::TopRight => "TR",
                    TileSubKey::BottomLeft => "BL",
                    TileSubKey::BottomRight => "BR",
                })
                .join("_")
        )
        .as_str(),
    ])
    .with_added_extension("jpg")
}

fn to_subdivision_key(key: &TileSubKey) -> SubDivisionKey {
    match key {
        TileSubKey::TopLeft => SubDivisionKey::TopLeft,
        TileSubKey::TopRight => SubDivisionKey::TopRight,
        TileSubKey::BottomLeft => SubDivisionKey::BottomLeft,
        TileSubKey::BottomRight => SubDivisionKey::BottomRight,
    }
}

fn reprojected(
    image: &RgbImage,
    projection: &impl Projection2D,
    gcs_bounds: DAabb2,
) -> Result<RgbImage, TileGenError> {
    let mut out = RgbImage::new(image.width(), image.height());

    let abs_bounds = DAabb2::new(
        projection.gcs_to_abs(gcs_bounds.min()),
        projection.gcs_to_abs(gcs_bounds.max()),
    );

    let image_size = DVec2::new(image.width() as f64, image.height() as f64);

    for (x, y, pixel) in out.enumerate_pixels_mut() {
        let rel = dvec2(0.0, 1.0) + dvec2(x as f64 + 0.5, -(y as f64 + 0.5)) / image_size;
        let abs_pos = abs_bounds.min() + abs_bounds.size() * rel;
        let gcs_pos = projection.abs_to_gcs(abs_pos);

        let img_pos_rel =
            dvec2(0.0, 1.0) + dvec2(1.0, -1.0) * (gcs_pos - gcs_bounds.min()) / gcs_bounds.size();

        *pixel =
            image::imageops::sample_bilinear(image, img_pos_rel.x as f32, img_pos_rel.y as f32)
                .ok_or(TileGenError::Reprojection)?;
    }

    Ok(out)
}

async fn produce_tile(
    state: &AppState,
    params: &GetTileRequest,
    dest_path: &PathBuf,
) -> Result<(), TileGenError> {
    let projection = match &params.projection {
        Projection::BoundedMercator { min_lat, max_lat } => BoundedMercatorProjection {
            lat_min: min_lat.to_radians(),
            lat_max: max_lat.to_radians(),
        },
    };

    let abs_bounds = projection.abs_bounds();
    let util_tile_key: utilities::glam_ext::sub_division::TileKey =
        params.tile_key.iter().map(to_subdivision_key).collect();

    let tile_abs_bbox = SubDivision2d { area: abs_bounds }.tile_bbox(&util_tile_key);
    let gcs_bbox_radians = DAabb2::new(
        projection.abs_to_gcs(tile_abs_bbox.min()),
        projection.abs_to_gcs(tile_abs_bbox.max()),
    );
    let gcs_bbox_degrees = DAabb2::new(
        gcs_bbox_radians.min().map(|v| v.to_degrees()),
        gcs_bbox_radians.max().map(|v| v.to_degrees()),
    );

    let img = match params.layer {
        Layer::Gibs(gibs_layer) => {
            fetch_epsg4326_gibs_image(
                &state.client,
                GibsEpsg4326Params {
                    layer: gibs_layer,
                    tile_params: Epsg4326TileParams {
                        resolution: (256, 256),
                        gcs_bounds: gcs_bbox_degrees,
                    },
                },
            )
            .await?
        }
        Layer::SenHub => {
            let token = get_sen_hub_bearer_token_cached(&state.client)
                .await
                .map_err(|_| TileGenError::RetryFailure)?;

            fetch_epsg4326_sen_hub_image(
                &state.client,
                token,
                Epsg4326TileParams {
                    resolution: (256, 256),
                    gcs_bounds: gcs_bbox_degrees,
                },
            )
            .await?
        }
        Layer::Local(local_layer) => {
            let rgba = fetch_epsg4326_local_image(
                &state.local_layer,
                local_layer,
                Epsg4326TileParams {
                    resolution: (256, 256),
                    gcs_bounds: gcs_bbox_degrees,
                },
            )
            .await?
            .unwrap_or(RgbaImage::new(256, 256));

            let (width, height) = rgba.dimensions();
            let mut rgb = RgbImage::new(width, height);

            for (x, y, pixel) in rgba.enumerate_pixels() {
                let [r, g, b, _a] = pixel.0;
                rgb.put_pixel(x, y, image::Rgb([r, g, b]));
            }

            rgb
        }
    };

    let img = reprojected(&img, &projection, gcs_bbox_radians)?;

    if let Some(parent) = dest_path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    img.save(dest_path)?;

    Ok(())
}

#[get("/tile")]
async fn get_tile_request(
    state: web::Data<AppState>,
    params: web::Json<GetTileRequest>,
) -> Result<fs::NamedFile, actix_web::Error> {
    let file_key = PathBuf::from(TILE_DIR).join(tile_sub_dir(&params.0));

    if !file_key.exists() {
        produce_tile(&state, &params.0, &file_key)
            .await
            .inspect_err(|err| panic!("Error producing tile: {}", err))
            .map_err(ErrorInternalServerError)?;
    }

    fs::NamedFile::open(&file_key).map_err(|_| ErrorNotFound("tile not found"))
}

pub async fn main() -> std::io::Result<()> {
    let host = std::env::var("HOST").unwrap();
    let port = std::env::var("PORT").unwrap().parse::<u16>().unwrap();
    let asset_root = PathBuf::from(std::env::var("ASSET_ROOT").unwrap());

    info!("Setting up state...");

    let state = web::Data::new(AppState {
        local_layer: LayeredDistributedMappedImage::from_directory(
            asset_root.join("local-layer/S2-MSI-L3-MCQ-2025-Q3"),
        )
        .unwrap(),
        client: reqwest::Client::new(),
    });

    info!("Starting server...");

    HttpServer::new(move || App::new().app_data(state.clone()).service(get_tile_request))
        .bind((host, port))?
        .run()
        .await
}
