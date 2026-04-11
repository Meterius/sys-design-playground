use utilities::glam_ext::bounding::{AxisAlignedBoundingBox2D, DAabb2};
use bevy::prelude::error;
use glam::DVec2;
use image::RgbImage;
use serde::Deserialize;
use serde_json::json;
use std::collections::HashMap;

pub const LAYER_MODIS_TERRA_CORRECTED_REFLECTANCE_TRUE_COLOR: &str =
    "MODIS_Terra_CorrectedReflectance_TrueColor";

#[derive(Debug)]
pub struct Epsg4326TileParams {
    pub resolution: (usize, usize),
    pub gcs_bounds: DAabb2,
}

pub struct GibsEpsg4326Params {
    pub layers: String,
    pub tile_params: Epsg4326TileParams,
}

fn fetch_gibs_epsg4326_url(params: &GibsEpsg4326Params) -> String {
    let DVec2 {
        x: min_lon,
        y: min_lat,
    } = params.tile_params.gcs_bounds.min();
    let DVec2 {
        x: max_lon,
        y: max_lat,
    } = params.tile_params.gcs_bounds.max();

    let (width, height) = params.tile_params.resolution;
    let layers = &params.layers;

    format!(
        "https://gibs.earthdata.nasa.gov/wms/epsg4326/best/wms.cgi?\
    version=1.3.0&service=WMS&request=GetMap&format=image/png\
    &STYLE=default&bbox={min_lat:.6},{min_lon:.6},{max_lat:.6},{max_lon:.6}&CRS=EPSG:4326\
    &HEIGHT={height}&WIDTH={width}\
    &TIME=2021-03-01&layers={layers}"
    )
}

pub async fn fetch_epsg4326_gibs_image(
    client: &reqwest::Client,
    params: GibsEpsg4326Params,
) -> Result<RgbImage, reqwest::Error> {
    let response = client
        .get(fetch_gibs_epsg4326_url(&params))
        .send()
        .await?
        .error_for_status()?;
    let bytes = response.bytes().await?.to_vec();
    let img = image::load_from_memory(&bytes).unwrap();
    Ok(img.to_rgb8())
}

pub async fn fetch_sen_hub_bearer_token(
    client: &reqwest::Client,
) -> Result<String, reqwest::Error> {
    let client_id = std::env::var("SEN_HUB_CLIENT_ID").unwrap_or("".into());
    let client_secret = std::env::var("SEN_HUB_CLIENT_SECRET").unwrap_or("".into());

    #[derive(Debug, Deserialize)]
    struct TokenResponse {
        access_token: String,
    }

    let mut params = HashMap::new();
    params.insert("grant_type", "client_credentials");
    params.insert("client_id", client_id.as_str());
    params.insert("client_secret", client_secret.as_str());

    let response = client
        .post("https://services.sentinel-hub.com/auth/realms/main/protocol/openid-connect/token")
        .form(&params)
        .send()
        .await?;

    let response_res = response.error_for_status_ref().err();

    if let Some(err) = response_res {
        error!(
            "Sentinel Image Fetch Failure; status={:?} body={:?}",
            err.status(),
            response.text().await?,
        );
        Err(err)
    } else {
        let token: TokenResponse = response.json().await?;
        Ok(token.access_token)
    }
}

pub async fn fetch_epsg4326_sen_hub_image(
    client: &reqwest::Client,
    bearer_token: String,
    params: Epsg4326TileParams,
) -> Result<RgbImage, reqwest::Error> {
    let bbox = [
        params.gcs_bounds.min().x,
        params.gcs_bounds.min().y,
        params.gcs_bounds.max().x,
        params.gcs_bounds.max().y,
    ];

    let time_from = "2025-04-01T00:00:00Z";
    let time_to = "2025-08-31T23:59:59Z";

    let data_type = "sentinel-2-l2a";

    let evalscript = r#"//VERSION=3
        function setup() {
          return {
            input: ["B02", "B03", "B04"],
            output: { bands: 3 }
          };
        }

        function evaluatePixel(sample) {
          return [2.5 * sample.B04, 2.5 * sample.B03, 2.5 * sample.B02];
        }"#;

    let data = json!({
        "input": {
            "bounds": {
                "bbox": bbox
            },
            "data": [{
                "dataFilter": {
                    "timeRange": {
                        "from": time_from,
                        "to": time_to
                    },
                    "maxCloudCoverage": 10,
                    "mosaickingOrder": "leastCC"
                },
                "processing": {
                  "upsampling": "BICUBIC",
                  "downsampling": "BICUBIC"
                },
                "type": data_type
            }]
        },
        "output": {
            "width": params.resolution.0,
            "height": params.resolution.1,
            "responses": [{
                "identifier": "default",
                "format": {
                    "type": "image/jpeg"
                }
            }]
        },
        "evalscript": evalscript
    });

    let response = client
        .post("https://services.sentinel-hub.com/api/v1/process")
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {}", bearer_token))
        .json(&data)
        .send()
        .await?;

    let response_res = response.error_for_status_ref().err();

    if let Some(err) = response_res {
        error!(
            "Sentinel Image Fetch Failure; status={:?} body={:?}",
            err.status(),
            response.text().await?
        );
        Err(err)
    } else {
        let img = image::load_from_memory(&response.bytes().await?).unwrap();
        Ok(img.to_rgb8())
    }
}
