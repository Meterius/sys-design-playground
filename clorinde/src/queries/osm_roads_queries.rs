// This file was generated with `clorinde`. Do not modify.

#[derive(Debug)]
pub struct UpsertRoadParams<T1: crate::BytesSql> {
    pub osm_id: i64,
    pub class: crate::types::RoadClass,
    pub category: crate::types::RoadClassCategory,
    pub oneway: crate::types::RoadOneway,
    pub max_speed: i32,
    pub layer: i32,
    pub is_bridge: bool,
    pub is_tunnel: bool,
    pub geom: T1,
}
#[derive(Debug, Clone, PartialEq)]
pub struct ListAllRoads {
    pub osm_id: i64,
    pub class: crate::types::RoadClass,
    pub category: crate::types::RoadClassCategory,
    pub oneway: crate::types::RoadOneway,
    pub max_speed: i32,
    pub layer: i32,
    pub is_bridge: bool,
    pub is_tunnel: bool,
    pub geom: Vec<u8>,
}
pub struct ListAllRoadsBorrowed<'a> {
    pub osm_id: i64,
    pub class: crate::types::RoadClass,
    pub category: crate::types::RoadClassCategory,
    pub oneway: crate::types::RoadOneway,
    pub max_speed: i32,
    pub layer: i32,
    pub is_bridge: bool,
    pub is_tunnel: bool,
    pub geom: &'a [u8],
}
impl<'a> From<ListAllRoadsBorrowed<'a>> for ListAllRoads {
    fn from(
        ListAllRoadsBorrowed {
            osm_id,
            class,
            category,
            oneway,
            max_speed,
            layer,
            is_bridge,
            is_tunnel,
            geom,
        }: ListAllRoadsBorrowed<'a>,
    ) -> Self {
        Self {
            osm_id,
            class,
            category,
            oneway,
            max_speed,
            layer,
            is_bridge,
            is_tunnel,
            geom: geom.into(),
        }
    }
}
use crate::client::async_::GenericClient;
use futures::{self, StreamExt, TryStreamExt};
pub struct ListAllRoadsQuery<'c, 'a, 's, C: GenericClient, T, const N: usize> {
    client: &'c C,
    params: [&'a (dyn postgres_types::ToSql + Sync); N],
    query: &'static str,
    cached: Option<&'s tokio_postgres::Statement>,
    extractor: fn(&tokio_postgres::Row) -> Result<ListAllRoadsBorrowed, tokio_postgres::Error>,
    mapper: fn(ListAllRoadsBorrowed) -> T,
}
impl<'c, 'a, 's, C, T: 'c, const N: usize> ListAllRoadsQuery<'c, 'a, 's, C, T, N>
where
    C: GenericClient,
{
    pub fn map<R>(
        self,
        mapper: fn(ListAllRoadsBorrowed) -> R,
    ) -> ListAllRoadsQuery<'c, 'a, 's, C, R, N> {
        ListAllRoadsQuery {
            client: self.client,
            params: self.params,
            query: self.query,
            cached: self.cached,
            extractor: self.extractor,
            mapper,
        }
    }
    pub async fn one(self) -> Result<T, tokio_postgres::Error> {
        let row =
            crate::client::async_::one(self.client, self.query, &self.params, self.cached).await?;
        Ok((self.mapper)((self.extractor)(&row)?))
    }
    pub async fn all(self) -> Result<Vec<T>, tokio_postgres::Error> {
        self.iter().await?.try_collect().await
    }
    pub async fn opt(self) -> Result<Option<T>, tokio_postgres::Error> {
        let opt_row =
            crate::client::async_::opt(self.client, self.query, &self.params, self.cached).await?;
        Ok(opt_row
            .map(|row| {
                let extracted = (self.extractor)(&row)?;
                Ok((self.mapper)(extracted))
            })
            .transpose()?)
    }
    pub async fn iter(
        self,
    ) -> Result<
        impl futures::Stream<Item = Result<T, tokio_postgres::Error>> + 'c,
        tokio_postgres::Error,
    > {
        let stream = crate::client::async_::raw(
            self.client,
            self.query,
            crate::slice_iter(&self.params),
            self.cached,
        )
        .await?;
        let mapped = stream
            .map(move |res| {
                res.and_then(|row| {
                    let extracted = (self.extractor)(&row)?;
                    Ok((self.mapper)(extracted))
                })
            })
            .into_stream();
        Ok(mapped)
    }
}
pub struct ListAllRoadsStmt(&'static str, Option<tokio_postgres::Statement>);
pub fn list_all_roads() -> ListAllRoadsStmt {
    ListAllRoadsStmt(
        "SELECT osm_id, class, category, oneway, max_speed, layer, is_bridge, is_tunnel, ST_asewkb(geom::geometry) as geom FROM osm_roads",
        None,
    )
}
impl ListAllRoadsStmt {
    pub async fn prepare<'a, C: GenericClient>(
        mut self,
        client: &'a C,
    ) -> Result<Self, tokio_postgres::Error> {
        self.1 = Some(client.prepare(self.0).await?);
        Ok(self)
    }
    pub fn bind<'c, 'a, 's, C: GenericClient>(
        &'s self,
        client: &'c C,
    ) -> ListAllRoadsQuery<'c, 'a, 's, C, ListAllRoads, 0> {
        ListAllRoadsQuery {
            client,
            params: [],
            query: self.0,
            cached: self.1.as_ref(),
            extractor:
                |row: &tokio_postgres::Row| -> Result<ListAllRoadsBorrowed, tokio_postgres::Error> {
                    Ok(ListAllRoadsBorrowed {
                        osm_id: row.try_get(0)?,
                        class: row.try_get(1)?,
                        category: row.try_get(2)?,
                        oneway: row.try_get(3)?,
                        max_speed: row.try_get(4)?,
                        layer: row.try_get(5)?,
                        is_bridge: row.try_get(6)?,
                        is_tunnel: row.try_get(7)?,
                        geom: row.try_get(8)?,
                    })
                },
            mapper: |it| ListAllRoads::from(it),
        }
    }
}
pub struct UpsertRoadStmt(&'static str, Option<tokio_postgres::Statement>);
pub fn upsert_road() -> UpsertRoadStmt {
    UpsertRoadStmt(
        "INSERT INTO osm_roads ( osm_id, class, category, oneway, max_speed, layer, is_bridge, is_tunnel, geom ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, st_setsrid(st_geomfromewkb($9), 4326)::geography) ON CONFLICT(osm_id) DO UPDATE SET (class, category, oneway, max_speed, layer, is_bridge, is_tunnel, geom) = (excluded.class, excluded.category, excluded.oneway, excluded.max_speed, excluded.layer, excluded.is_bridge, excluded.is_tunnel, excluded.geom)",
        None,
    )
}
impl UpsertRoadStmt {
    pub async fn prepare<'a, C: GenericClient>(
        mut self,
        client: &'a C,
    ) -> Result<Self, tokio_postgres::Error> {
        self.1 = Some(client.prepare(self.0).await?);
        Ok(self)
    }
    pub async fn bind<'c, 'a, 's, C: GenericClient, T1: crate::BytesSql>(
        &'s self,
        client: &'c C,
        osm_id: &'a i64,
        class: &'a crate::types::RoadClass,
        category: &'a crate::types::RoadClassCategory,
        oneway: &'a crate::types::RoadOneway,
        max_speed: &'a i32,
        layer: &'a i32,
        is_bridge: &'a bool,
        is_tunnel: &'a bool,
        geom: &'a T1,
    ) -> Result<u64, tokio_postgres::Error> {
        client
            .execute(
                self.0,
                &[
                    osm_id, class, category, oneway, max_speed, layer, is_bridge, is_tunnel, geom,
                ],
            )
            .await
    }
}
impl<'a, C: GenericClient + Send + Sync, T1: crate::BytesSql>
    crate::client::async_::Params<
        'a,
        'a,
        'a,
        UpsertRoadParams<T1>,
        std::pin::Pin<
            Box<dyn futures::Future<Output = Result<u64, tokio_postgres::Error>> + Send + 'a>,
        >,
        C,
    > for UpsertRoadStmt
{
    fn params(
        &'a self,
        client: &'a C,
        params: &'a UpsertRoadParams<T1>,
    ) -> std::pin::Pin<
        Box<dyn futures::Future<Output = Result<u64, tokio_postgres::Error>> + Send + 'a>,
    > {
        Box::pin(self.bind(
            client,
            &params.osm_id,
            &params.class,
            &params.category,
            &params.oneway,
            &params.max_speed,
            &params.layer,
            &params.is_bridge,
            &params.is_tunnel,
            &params.geom,
        ))
    }
}
pub struct UpsertRoadsStreamingStartStmt(&'static str, Option<tokio_postgres::Statement>);
pub fn upsert_roads_streaming_start() -> UpsertRoadsStreamingStartStmt {
    UpsertRoadsStreamingStartStmt(
        "CREATE TEMP TABLE tmp_upsert_roads_streaming AS SELECT * FROM osm_roads LIMIT 0",
        None,
    )
}
impl UpsertRoadsStreamingStartStmt {
    pub async fn prepare<'a, C: GenericClient>(
        mut self,
        client: &'a C,
    ) -> Result<Self, tokio_postgres::Error> {
        self.1 = Some(client.prepare(self.0).await?);
        Ok(self)
    }
    pub async fn bind<'c, 'a, 's, C: GenericClient>(
        &'s self,
        client: &'c C,
    ) -> Result<u64, tokio_postgres::Error> {
        client.execute(self.0, &[]).await
    }
}
pub struct UpsertRoadStreamingTransferStmt(&'static str, Option<tokio_postgres::Statement>);
pub fn upsert_road_streaming_transfer() -> UpsertRoadStreamingTransferStmt {
    UpsertRoadStreamingTransferStmt(
        "COPY tmp_upsert_roads_streaming ( osm_id, class, category, oneway, max_speed, layer, is_bridge, is_tunnel, geom ) FROM stdin binary",
        None,
    )
}
impl UpsertRoadStreamingTransferStmt {
    pub async fn prepare<'a, C: GenericClient>(
        mut self,
        client: &'a C,
    ) -> Result<Self, tokio_postgres::Error> {
        self.1 = Some(client.prepare(self.0).await?);
        Ok(self)
    }
    pub async fn bind<'c, 'a, 's, C: GenericClient>(
        &'s self,
        client: &'c C,
    ) -> Result<u64, tokio_postgres::Error> {
        client.execute(self.0, &[]).await
    }
}
pub struct UpsertRoadsStreamingCommitStmt(&'static str, Option<tokio_postgres::Statement>);
pub fn upsert_roads_streaming_commit() -> UpsertRoadsStreamingCommitStmt {
    UpsertRoadsStreamingCommitStmt(
        "INSERT INTO osm_roads ( osm_id, class, category, oneway, max_speed, layer, is_bridge, is_tunnel, geom ) SELECT s.osm_id, s.class, s.category, s.oneway, s.max_speed, s.layer, s.is_bridge, s.is_tunnel, st_setsrid(st_geomfromewkb(s.geom), 4326)::geography FROM tmp_upsert_roads_streaming s ON CONFLICT(osm_id) DO UPDATE SET (class, category, oneway, max_speed, layer, is_bridge, is_tunnel, geom) = (excluded.class, excluded.category, excluded.oneway, excluded.max_speed, excluded.layer, excluded.is_bridge, excluded.is_tunnel, excluded.geom)",
        None,
    )
}
impl UpsertRoadsStreamingCommitStmt {
    pub async fn prepare<'a, C: GenericClient>(
        mut self,
        client: &'a C,
    ) -> Result<Self, tokio_postgres::Error> {
        self.1 = Some(client.prepare(self.0).await?);
        Ok(self)
    }
    pub async fn bind<'c, 'a, 's, C: GenericClient>(
        &'s self,
        client: &'c C,
    ) -> Result<u64, tokio_postgres::Error> {
        client.execute(self.0, &[]).await
    }
}
pub struct UpsertRoadsStreamingEndStmt(&'static str, Option<tokio_postgres::Statement>);
pub fn upsert_roads_streaming_end() -> UpsertRoadsStreamingEndStmt {
    UpsertRoadsStreamingEndStmt("DROP TABLE tmp_upsert_roads_streaming", None)
}
impl UpsertRoadsStreamingEndStmt {
    pub async fn prepare<'a, C: GenericClient>(
        mut self,
        client: &'a C,
    ) -> Result<Self, tokio_postgres::Error> {
        self.1 = Some(client.prepare(self.0).await?);
        Ok(self)
    }
    pub async fn bind<'c, 'a, 's, C: GenericClient>(
        &'s self,
        client: &'c C,
    ) -> Result<u64, tokio_postgres::Error> {
        client.execute(self.0, &[]).await
    }
}
