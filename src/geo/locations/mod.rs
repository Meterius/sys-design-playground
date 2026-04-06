use bevy::prelude::error;
use glam::{DVec2, dvec2};
use std::env;
use std::sync::Arc;
use thiserror::Error;
use tokio_postgres::NoTls;

#[derive(Debug)]
pub struct Location {
    pub pos: DVec2,
    pub name: String,
}

pub struct LocationClient {
    client: tokio_postgres::Client,
    #[warn(unused)]
    connection_handle: tokio::task::JoinHandle<()>,
}

#[derive(Debug, Error)]
pub enum LocationClientError {
    #[error("Postgres Error: {0:?}")]
    PostgresError(#[from] tokio_postgres::Error),
    #[error("Missing Environment Variables")]
    MissingEnvironmentVariables,
}

impl LocationClient {
    pub async fn connect() -> Result<LocationClient, LocationClientError> {
        let db_name = env::var("INFRA_GEO_POSTGRES_DB_NAME")
            .map_err(|_| LocationClientError::MissingEnvironmentVariables)?;
        let host = env::var("INFRA_GEO_POSTGRES_HOST")
            .map_err(|_| LocationClientError::MissingEnvironmentVariables)?;
        let user = env::var("INFRA_GEO_POSTGRES_USER")
            .map_err(|_| LocationClientError::MissingEnvironmentVariables)?;
        let password = env::var("INFRA_GEO_POSTGRES_PASSWORD")
            .map_err(|_| LocationClientError::MissingEnvironmentVariables)?;

        let (client, connection) = tokio_postgres::connect(
            &format!("dbname={db_name} host={host} user={user} password={password}"),
            NoTls,
        )
        .await?;

        Ok(Self {
            client,
            connection_handle: tokio::task::spawn(async {
                let _ = connection.await.inspect_err(|err| {
                    error!("Connection error: {err}");
                });
            }),
        })
    }
    pub async fn fetch_nearest(
        &self,
        pos: DVec2,
        limit: usize,
    ) -> Result<Vec<Location>, LocationClientError> {
        let rows = self
            .client
            .query(
                "SELECT name, longitude, latitude FROM locations
WHERE ST_DWithin(geom, ST_SetSRID(ST_MakePoint($1, $2), 4326)::geography, $3)
ORDER BY ST_DISTANCE(geom, ST_SetSRID(ST_MakePoint($1, $2), 4326)::geography)
LIMIT 100;",
                &[&(pos.x as f64), &(pos.y as f64), &300.0f64],
            )
            .await?;

        (rows.into_iter().map(|row| {
            Ok(Location {
                name: row.try_get(0)?,
                pos: dvec2(row.try_get::<usize, f64>(1)?, row.try_get::<usize, f64>(2)?),
            })
        }))
        .collect()
    }
}
