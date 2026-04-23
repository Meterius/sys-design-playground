use crate::app::utils::async_requests::{
    AsyncRequestsPlugin, Request, RequestClient, RequestKind, RequestManager,
};
use crate::geo::coords::BoundedMercatorProjection;
use backend_model::earth_tiling_service_model::{
    GetTileRequest, GibsLayer, Layer, LocalLayer, TileSubKey,
};
use bevy::app::{App, Startup};
use bevy::prelude::{Entity, Plugin, Resource, World};
use itertools::Itertools;
use ratelimit::Ratelimiter;
use reqwest::Url;
use std::collections::HashMap;
use std::env;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use thiserror::Error;
use tracing::debug;
use utilities::glam_ext::sub_division::{SubDivisionKey, TileKey};

pub struct TilingRequestPlugin {}

impl Plugin for TilingRequestPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(AsyncRequestsPlugin::<
            TileImageRequestKind,
            TileImageRequestClient,
        >::new())
            .add_systems(Startup, startup);
    }
}

#[derive(Resource)]
pub struct TileRequestManagersByDataset {
    pub managers: HashMap<Layer, Entity>,
}

fn startup(world: &mut World) {
    let api_url = Url::from_str(&env::var("INFRA_GEO_API_URL").unwrap()).unwrap();

    let mut managers = HashMap::new();
    let client = reqwest::Client::new();

    let mut add_layer = |layer: Layer, max_concurrent: usize, rate: u64| {
        managers.insert(
            layer,
            world
                .commands()
                .spawn(RequestManager::new(
                    max_concurrent,
                    Some(Ratelimiter::new(rate)),
                    TileImageRequestClient {
                        url: api_url.clone(),
                        connection_refused: Arc::new(AtomicBool::new(false)),
                        layer,
                        client: client.clone(),
                    },
                ))
                .id(),
        )
    };

    add_layer(Layer::SenHub, 10, 4);
    add_layer(Layer::Local(LocalLayer::GlobalMosaicSen2), 20, 40);
    add_layer(
        Layer::Gibs(GibsLayer::LayerModisTerraCorrectedReflectanceTrueColor),
        40,
        20,
    );

    world.insert_resource(TileRequestManagersByDataset { managers });
}

#[derive(Debug, Error)]
pub enum TileImageRequestError {
    #[error("Http error: {0:?}")]
    Http(#[from] reqwest::Error),
    #[error("Serialization error: {0:?}")]
    Serialization(#[from] serde_json::Error),
    #[error("IO error: {0:?}")]
    Io(#[from] tokio::io::Error),
}

#[derive(Clone)]
pub struct TileImageRequestClient {
    pub url: Url,
    pub client: reqwest::Client,
    pub layer: Layer,
    pub connection_refused: Arc<AtomicBool>,
}

impl TileImageRequestClient {
    fn get_tile_dir() -> PathBuf {
        PathBuf::from_iter(["assets", "cache", "tile_image_requests"])
    }

    fn get_asset_tile_dir() -> PathBuf {
        PathBuf::from_iter(["cache", "tile_image_requests"])
    }

    fn get_tile_path(tile_key: &TileKey) -> bevy::prelude::Result<PathBuf, TileImageRequestError> {
        let tile_dir = Self::get_tile_dir();
        let key_hex = format!(
            "tile_{}",
            tile_key
                .iter()
                .map(|sub_key| match sub_key {
                    SubDivisionKey::TopLeft => "TL",
                    SubDivisionKey::TopRight => "TR",
                    SubDivisionKey::BottomLeft => "BL",
                    SubDivisionKey::BottomRight => "BR",
                })
                .join("_")
        );

        Ok(tile_dir.join(key_hex).with_added_extension("jpg"))
    }

    fn get_asset_tile_path(
        tile_key: &TileKey,
    ) -> bevy::prelude::Result<PathBuf, TileImageRequestError> {
        let tile_dir = Self::get_asset_tile_dir();
        let key_hex = format!(
            "tile_{}",
            tile_key
                .iter()
                .map(|sub_key| match sub_key {
                    SubDivisionKey::TopLeft => "TL",
                    SubDivisionKey::TopRight => "TR",
                    SubDivisionKey::BottomLeft => "BL",
                    SubDivisionKey::BottomRight => "BR",
                })
                .join("_")
        );

        Ok(tile_dir.join(key_hex).with_added_extension("jpg"))
    }
}

impl RequestClient<TileImageRequestKind> for TileImageRequestClient {
    async fn fetch_preflight(
        &self,
        key: &<TileImageRequestKind as RequestKind>::Key,
    ) -> bevy::prelude::Result<Option<Option<PathBuf>>, TileImageRequestError> {
        let tile_path = Self::get_tile_path(&key.1)?;
        if tokio::fs::try_exists(&tile_path).await? {
            Ok(Some(Some(Self::get_asset_tile_path(&key.1)?)))
        } else {
            Ok(None)
        }
    }

    async fn fetch(
        &self,
        key: &<TileImageRequestKind as RequestKind>::Key,
    ) -> bevy::prelude::Result<Option<PathBuf>, TileImageRequestError> {
        if self
            .connection_refused
            .load(std::sync::atomic::Ordering::Relaxed)
        {
            return Ok(None);
        }

        let res = self
            .client
            .get(self.url.clone())
            .json(&GetTileRequest {
                tile_key: backend_model::earth_tiling_service_model::TileKey::from_iter(
                    key.1.iter().map(|x| match x {
                        SubDivisionKey::BottomLeft => TileSubKey::BottomLeft,
                        SubDivisionKey::BottomRight => TileSubKey::BottomRight,
                        SubDivisionKey::TopLeft => TileSubKey::TopLeft,
                        SubDivisionKey::TopRight => TileSubKey::TopRight,
                    }),
                ),
                projection:
                    backend_model::earth_tiling_service_model::Projection::BoundedMercator {
                        min_lat: key.0.lat_min.to_degrees(),
                        max_lat: key.0.lat_max.to_degrees(),
                    },
                layer: self.layer,
            })
            .send()
            .await
            .inspect_err(|err| {
                if err.is_connect() {
                    self.connection_refused
                        .store(true, std::sync::atomic::Ordering::Relaxed);
                }
            })?
            .error_for_status()
            .map(Some)
            .or_else(|err| {
                if err.status() == Some(reqwest::StatusCode::NOT_FOUND) {
                    Ok(None)
                } else {
                    Err(err)
                }
            })?;

        if let Some(res) = res {
            let tile_path = Self::get_tile_path(&key.1)?;

            tokio::fs::create_dir_all(&tile_path.parent().unwrap()).await?;
            tokio::fs::write(&tile_path, res.bytes().await?).await?;

            debug!("Fetched tile: {:?}", &tile_path);
            debug!("Asset path: {:?}", Self::get_asset_tile_path(&key.1));
            Ok(Some(Self::get_asset_tile_path(&key.1)?))
        } else {
            Ok(None)
        }
    }
}

pub struct TileImageRequestKind {}

impl RequestKind for TileImageRequestKind {
    type Key = (BoundedMercatorProjection, TileKey);
    type Value = Option<PathBuf>;
    type Error = TileImageRequestError;
}

pub type TileImageRequest = Request<TileImageRequestKind>;
