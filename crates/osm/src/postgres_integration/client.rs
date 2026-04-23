use crate::model::road::{Road, RoadClassCategory};
use futures::StreamExt;
use generated_queries::queries::osm_roads_queries::{
    fetch_roads_by_area, fetch_roads_by_area_and_category,
};
use generated_queries::tokio_postgres;
use geojson::FeatureCollection;
use glam::{DVec2, dvec2};
use postgis::ewkb::{AsEwkbPolygon, EwkbRead, EwkbWrite, LineString, Point, Polygon};
use std::env;
use thiserror::Error;
use tokio_postgres::NoTls;
use tracing::error;
use utilities::glam_ext::bounding::{AxisAlignedBoundingBox2D, DAabb2};

const GEO_FABRIK_INDEX: &str = "https://download.geofabrik.de/index-v1.json";

fn dvec2_to_point(point: DVec2) -> Point {
    Point {
        srid: None,
        x: point.x,
        y: point.y,
    }
}

fn point_to_dvec2(point: Point) -> DVec2 {
    dvec2(point.x, point.y)
}

fn ewkb_to_vec(source: impl EwkbWrite) -> Result<Vec<u8>, OsmError> {
    let mut data = Vec::new();
    source.write_ewkb(&mut data)?;
    Ok(data)
}

#[derive(Debug, Error)]
pub enum OsmError {
    #[error("Request Error: {0}")]
    ReqwestError(#[from] reqwest::Error),
    #[error("Postgis Error: {0}")]
    PostgisError(#[from] postgis::error::Error),
    #[error("Postgres Error: {0}")]
    PostgresError(#[from] tokio_postgres::Error),
    #[error("Env Error: {0}")]
    EnvError(#[from] env::VarError),
}

pub async fn fetch_fabrik_index(client: &reqwest::Client) -> Result<FeatureCollection, OsmError> {
    client
        .get(GEO_FABRIK_INDEX)
        .send()
        .await?
        .json()
        .await
        .map_err(OsmError::ReqwestError)
}

pub struct OsmClientConfig {
    pub database_config: tokio_postgres::Config,
}

pub struct OsmClient {
    client: tokio_postgres::Client,
    _connection_handle: tokio::task::JoinHandle<()>,
}

impl OsmClient {
    pub async fn connect(config: OsmClientConfig) -> Result<OsmClient, OsmError> {
        let (client, connection) = config
            .database_config
            .connect(
                // format!(
                //     "user={} password={} host={} dbname={}",
                //     env::var("INFRA_GEO_POSTGRES_USER")?,
                //     env::var("INFRA_GEO_POSTGRES_PASSWORD")?,
                //     env::var("INFRA_GEO_POSTGRES_HOST")?,
                //     env::var("INFRA_GEO_POSTGRES_DB_NAME")?
                // )
                // .as_str(),
                NoTls,
            )
            .await?;

        Ok(Self {
            client,
            _connection_handle: tokio::spawn(async move {
                let _ = connection.await.inspect_err(|err| {
                    error!("Connection Error: {:?}", err);
                });
            }),
        })
    }

    pub async fn fetch_roads(
        &self,
        bounds: DAabb2,
    ) -> Result<impl futures::Stream<Item = Result<Road, OsmError>>, OsmError> {
        let corners = bounds.corners().map(dvec2_to_point).collect::<Vec<_>>();

        let iter = fetch_roads_by_area()
            .bind(
                &self.client,
                &ewkb_to_vec(
                    Polygon {
                        rings: vec![LineString {
                            points: std::iter::once(corners[corners.len() - 1])
                                .chain(corners.into_iter())
                                .collect(),
                            srid: None,
                        }],
                        srid: None,
                    }
                    .as_ewkb(),
                )?,
            )
            .iter()
            .await?;

        Ok(iter.map(|r| {
            r.map_err(OsmError::from).and_then(|data| {
                Ok(Road {
                    oneway: data.oneway.into(),
                    osm_id: data.osm_id,
                    is_bridge: data.is_bridge,
                    is_tunnel: data.is_tunnel,
                    max_speed: data.max_speed.map(|v| v as u32),
                    class: data.class.into(),
                    reference: data.reference,
                    layer: data.layer,
                    geometry: LineString::read_ewkb(&mut std::io::Cursor::new(data.geom))?
                        .points
                        .into_iter()
                        .map(point_to_dvec2)
                        .collect(),
                })
            })
        }))
    }

    pub async fn fetch_roads_by_category(
        &self,
        bounds: DAabb2,
        category: RoadClassCategory,
    ) -> Result<impl futures::Stream<Item = Result<Road, OsmError>>, OsmError> {
        let corners = bounds.corners().map(dvec2_to_point).collect::<Vec<_>>();

        let iter = fetch_roads_by_area_and_category()
            .bind(
                &self.client,
                &category.into(),
                &ewkb_to_vec(
                    Polygon {
                        rings: vec![LineString {
                            points: std::iter::once(corners[corners.len() - 1])
                                .chain(corners.into_iter())
                                .collect(),
                            srid: None,
                        }],
                        srid: None,
                    }
                    .as_ewkb(),
                )?,
            )
            .iter()
            .await?;

        Ok(iter.map(|r| {
            r.map_err(OsmError::from).and_then(|data| {
                Ok(Road {
                    oneway: data.oneway.into(),
                    osm_id: data.osm_id,
                    is_bridge: data.is_bridge,
                    is_tunnel: data.is_tunnel,
                    max_speed: data.max_speed.map(|v| v as u32),
                    class: data.class.into(),
                    reference: data.reference,
                    layer: data.layer,
                    geometry: LineString::read_ewkb(&mut std::io::Cursor::new(data.geom))?
                        .points
                        .into_iter()
                        .map(point_to_dvec2)
                        .collect(),
                })
            })
        }))
    }
}
