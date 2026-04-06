use crate::geo::coords::LonLatVec2;
use bevy::prelude::error;
use std::sync::Arc;
use thiserror::Error;
use tokio_postgres::NoTls;

#[derive(Debug)]
pub struct Location {
    pub pos: LonLatVec2,
    pub name: String,
}

pub struct LocationClient {
    client: tokio_postgres::Client,
    connection_handle: tokio::task::JoinHandle<()>,
}

#[derive(Debug, Error)]
pub enum LocationClientError {
    #[error("Postgres Error: {0:?}")]
    PostgresError(#[from] tokio_postgres::Error),
}

impl LocationClient {
    pub async fn connect() -> Result<LocationClient, LocationClientError> {
        let (client, connection) =
            tokio_postgres::connect("dbname=app_db host=localhost user=dev password=dev", NoTls)
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
        pos: LonLatVec2,
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
                pos: LonLatVec2 {
                    x: row.try_get::<usize, f64>(1)? as f32,
                    y: row.try_get::<usize, f64>(2)? as f32,
                },
            })
        }))
        .collect()
    }
}
