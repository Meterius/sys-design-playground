use crate::app::utils::async_requests::{
    AsyncRequestsPlugin, Request, RequestClient, RequestKind, RequestManager,
};
use crate::geo::coords::BoundedMercatorProjection;
use backend_model::earth_tiling_service_model::{
    GetTileRequest, GibsLayer, Layer, LocalLayer, TileSubKey,
};
use bevy::app::{App, Startup};
use bevy::prelude::{Entity, Plugin, Resource, World};
use ratelimit::Ratelimiter;
use std::collections::HashMap;
use std::path::PathBuf;
use thiserror::Error;
use tracing::info;
use utilities::glam_ext::sub_division::{SubDivisionKey, TileKey};

pub struct TileRequestsPlugin {}

impl Plugin for TileRequestsPlugin {
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
                        layer,
                        client: client.clone(),
                    },
                ))
                .id(),
        )
    };

    add_layer(Layer::SenHub, 40, 4);
    add_layer(Layer::Local(LocalLayer::GlobalMosaicSen2), 40, 40);
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
    pub client: reqwest::Client,
    pub layer: Layer,
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
        let key_hex = hex::encode(serde_json::to_string(&tile_key)?);
        Ok(tile_dir.join(key_hex).with_added_extension("jpg"))
    }

    fn get_asset_tile_path(tile_key: &TileKey) -> bevy::prelude::Result<PathBuf, TileImageRequestError> {
        let tile_dir = Self::get_asset_tile_dir();
        let key_hex = hex::encode(serde_json::to_string(&tile_key)?);
        Ok(tile_dir.join(key_hex).with_added_extension("jpg"))
    }
}

impl RequestClient<TileImageRequestKind> for TileImageRequestClient {
    async fn fetch_preflight(
        &self,
        key: &<TileImageRequestKind as RequestKind>::Key,
    ) -> bevy::prelude::Result<Option<PathBuf>, TileImageRequestError> {
        let tile_path = Self::get_tile_path(&key.1)?;
        if tokio::fs::try_exists(&tile_path).await? {
            Ok(Some(Self::get_asset_tile_path(&key.1)?))
        } else {
            Ok(None)
        }
    }

    async fn fetch(
        &self,
        key: &<TileImageRequestKind as RequestKind>::Key,
    ) -> bevy::prelude::Result<PathBuf, TileImageRequestError> {
        let res = self
            .client
            .get("http://localhost:80/tile")
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
            .await?
            .error_for_status()?;

        let tile_path = Self::get_tile_path(&key.1)?;

        tokio::fs::create_dir_all(&tile_path.parent().unwrap()).await?;
        tokio::fs::write(&tile_path, res.bytes().await?).await?;

        info!("Fetched tile: {:?}", &tile_path);
        info!("Asset path: {:?}", Self::get_asset_tile_path(&key.1));
        Self::get_asset_tile_path(&key.1)
    }
}

pub struct TileImageRequestKind {}

impl RequestKind for TileImageRequestKind {
    type Key = (BoundedMercatorProjection, TileKey);
    type Value = PathBuf;
    type Error = TileImageRequestError;
}

pub type TileImageRequest = Request<TileImageRequestKind>;

pub type TileImageRequestManager = RequestManager<TileImageRequestKind, TileImageRequestClient>;
