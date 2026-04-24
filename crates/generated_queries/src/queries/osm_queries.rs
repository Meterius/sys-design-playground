// This file was generated with `clorinde`. Do not modify.

#[derive(Debug)]
pub struct FetchRoadsByAreaAndCategoryParams<T1: crate::BytesSql> {
    pub category: crate::types::RoadClassCategory,
    pub bounds: T1,
}
#[derive(Debug)]
pub struct UpsertRoadParams<T1: crate::BytesSql> {
    pub osm_id: i64,
    pub class: crate::types::RoadClass,
    pub category: crate::types::RoadClassCategory,
    pub oneway: crate::types::RoadOneway,
    pub max_speed: Option<i32>,
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
    pub reference: String,
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
    pub reference: &'a str,
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
            reference,
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
            reference: reference.into(),
            is_bridge,
            is_tunnel,
            geom: geom.into(),
        }
    }
}
#[derive(Debug, Clone, PartialEq)]
pub struct FetchRoadsByArea {
    pub osm_id: i64,
    pub class: crate::types::RoadClass,
    pub category: crate::types::RoadClassCategory,
    pub oneway: crate::types::RoadOneway,
    pub max_speed: Option<i32>,
    pub layer: i32,
    pub reference: String,
    pub is_bridge: bool,
    pub is_tunnel: bool,
    pub geom: Vec<u8>,
}
pub struct FetchRoadsByAreaBorrowed<'a> {
    pub osm_id: i64,
    pub class: crate::types::RoadClass,
    pub category: crate::types::RoadClassCategory,
    pub oneway: crate::types::RoadOneway,
    pub max_speed: Option<i32>,
    pub layer: i32,
    pub reference: &'a str,
    pub is_bridge: bool,
    pub is_tunnel: bool,
    pub geom: &'a [u8],
}
impl<'a> From<FetchRoadsByAreaBorrowed<'a>> for FetchRoadsByArea {
    fn from(
        FetchRoadsByAreaBorrowed {
            osm_id,
            class,
            category,
            oneway,
            max_speed,
            layer,
            reference,
            is_bridge,
            is_tunnel,
            geom,
        }: FetchRoadsByAreaBorrowed<'a>,
    ) -> Self {
        Self {
            osm_id,
            class,
            category,
            oneway,
            max_speed,
            layer,
            reference: reference.into(),
            is_bridge,
            is_tunnel,
            geom: geom.into(),
        }
    }
}
#[derive(Debug, Clone, PartialEq)]
pub struct FetchRoadsByAreaAndCategory {
    pub osm_id: i64,
    pub class: crate::types::RoadClass,
    pub category: crate::types::RoadClassCategory,
    pub oneway: crate::types::RoadOneway,
    pub max_speed: Option<i32>,
    pub layer: i32,
    pub reference: String,
    pub is_bridge: bool,
    pub is_tunnel: bool,
    pub geom: Vec<u8>,
}
pub struct FetchRoadsByAreaAndCategoryBorrowed<'a> {
    pub osm_id: i64,
    pub class: crate::types::RoadClass,
    pub category: crate::types::RoadClassCategory,
    pub oneway: crate::types::RoadOneway,
    pub max_speed: Option<i32>,
    pub layer: i32,
    pub reference: &'a str,
    pub is_bridge: bool,
    pub is_tunnel: bool,
    pub geom: &'a [u8],
}
impl<'a> From<FetchRoadsByAreaAndCategoryBorrowed<'a>> for FetchRoadsByAreaAndCategory {
    fn from(
        FetchRoadsByAreaAndCategoryBorrowed {
            osm_id,
            class,
            category,
            oneway,
            max_speed,
            layer,
            reference,
            is_bridge,
            is_tunnel,
            geom,
        }: FetchRoadsByAreaAndCategoryBorrowed<'a>,
    ) -> Self {
        Self {
            osm_id,
            class,
            category,
            oneway,
            max_speed,
            layer,
            reference: reference.into(),
            is_bridge,
            is_tunnel,
            geom: geom.into(),
        }
    }
}
#[derive(Debug, Clone, PartialEq)]
pub struct FetchBuildingsByArea {
    pub osm_id: i64,
    pub kind: Option<String>,
    pub geom: Vec<u8>,
}
pub struct FetchBuildingsByAreaBorrowed<'a> {
    pub osm_id: i64,
    pub kind: Option<&'a str>,
    pub geom: &'a [u8],
}
impl<'a> From<FetchBuildingsByAreaBorrowed<'a>> for FetchBuildingsByArea {
    fn from(
        FetchBuildingsByAreaBorrowed { osm_id, kind, geom }: FetchBuildingsByAreaBorrowed<'a>,
    ) -> Self {
        Self {
            osm_id,
            kind: kind.map(|v| v.into()),
            geom: geom.into(),
        }
    }
}
#[derive(Debug, Clone, PartialEq)]
pub struct FetchWatersByArea {
    pub osm_id: i64,
    pub class: crate::types::WaterClass,
    pub geom: Vec<u8>,
}
pub struct FetchWatersByAreaBorrowed<'a> {
    pub osm_id: i64,
    pub class: crate::types::WaterClass,
    pub geom: &'a [u8],
}
impl<'a> From<FetchWatersByAreaBorrowed<'a>> for FetchWatersByArea {
    fn from(
        FetchWatersByAreaBorrowed {
            osm_id,
            class,
            geom,
        }: FetchWatersByAreaBorrowed<'a>,
    ) -> Self {
        Self {
            osm_id,
            class,
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
pub struct FetchRoadsByAreaQuery<'c, 'a, 's, C: GenericClient, T, const N: usize> {
    client: &'c C,
    params: [&'a (dyn postgres_types::ToSql + Sync); N],
    query: &'static str,
    cached: Option<&'s tokio_postgres::Statement>,
    extractor: fn(&tokio_postgres::Row) -> Result<FetchRoadsByAreaBorrowed, tokio_postgres::Error>,
    mapper: fn(FetchRoadsByAreaBorrowed) -> T,
}
impl<'c, 'a, 's, C, T: 'c, const N: usize> FetchRoadsByAreaQuery<'c, 'a, 's, C, T, N>
where
    C: GenericClient,
{
    pub fn map<R>(
        self,
        mapper: fn(FetchRoadsByAreaBorrowed) -> R,
    ) -> FetchRoadsByAreaQuery<'c, 'a, 's, C, R, N> {
        FetchRoadsByAreaQuery {
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
pub struct FetchRoadsByAreaAndCategoryQuery<'c, 'a, 's, C: GenericClient, T, const N: usize> {
    client: &'c C,
    params: [&'a (dyn postgres_types::ToSql + Sync); N],
    query: &'static str,
    cached: Option<&'s tokio_postgres::Statement>,
    extractor: fn(
        &tokio_postgres::Row,
    ) -> Result<FetchRoadsByAreaAndCategoryBorrowed, tokio_postgres::Error>,
    mapper: fn(FetchRoadsByAreaAndCategoryBorrowed) -> T,
}
impl<'c, 'a, 's, C, T: 'c, const N: usize> FetchRoadsByAreaAndCategoryQuery<'c, 'a, 's, C, T, N>
where
    C: GenericClient,
{
    pub fn map<R>(
        self,
        mapper: fn(FetchRoadsByAreaAndCategoryBorrowed) -> R,
    ) -> FetchRoadsByAreaAndCategoryQuery<'c, 'a, 's, C, R, N> {
        FetchRoadsByAreaAndCategoryQuery {
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
pub struct FetchBuildingsByAreaQuery<'c, 'a, 's, C: GenericClient, T, const N: usize> {
    client: &'c C,
    params: [&'a (dyn postgres_types::ToSql + Sync); N],
    query: &'static str,
    cached: Option<&'s tokio_postgres::Statement>,
    extractor:
        fn(&tokio_postgres::Row) -> Result<FetchBuildingsByAreaBorrowed, tokio_postgres::Error>,
    mapper: fn(FetchBuildingsByAreaBorrowed) -> T,
}
impl<'c, 'a, 's, C, T: 'c, const N: usize> FetchBuildingsByAreaQuery<'c, 'a, 's, C, T, N>
where
    C: GenericClient,
{
    pub fn map<R>(
        self,
        mapper: fn(FetchBuildingsByAreaBorrowed) -> R,
    ) -> FetchBuildingsByAreaQuery<'c, 'a, 's, C, R, N> {
        FetchBuildingsByAreaQuery {
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
pub struct FetchWatersByAreaQuery<'c, 'a, 's, C: GenericClient, T, const N: usize> {
    client: &'c C,
    params: [&'a (dyn postgres_types::ToSql + Sync); N],
    query: &'static str,
    cached: Option<&'s tokio_postgres::Statement>,
    extractor: fn(&tokio_postgres::Row) -> Result<FetchWatersByAreaBorrowed, tokio_postgres::Error>,
    mapper: fn(FetchWatersByAreaBorrowed) -> T,
}
impl<'c, 'a, 's, C, T: 'c, const N: usize> FetchWatersByAreaQuery<'c, 'a, 's, C, T, N>
where
    C: GenericClient,
{
    pub fn map<R>(
        self,
        mapper: fn(FetchWatersByAreaBorrowed) -> R,
    ) -> FetchWatersByAreaQuery<'c, 'a, 's, C, R, N> {
        FetchWatersByAreaQuery {
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
        "SELECT osm_id, class, category, oneway, max_speed, layer, reference, is_bridge, is_tunnel, ST_asewkb(geom::geometry) as geom FROM osm_roads",
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
                        reference: row.try_get(6)?,
                        is_bridge: row.try_get(7)?,
                        is_tunnel: row.try_get(8)?,
                        geom: row.try_get(9)?,
                    })
                },
            mapper: |it| ListAllRoads::from(it),
        }
    }
}
pub struct FetchRoadsByAreaStmt(&'static str, Option<tokio_postgres::Statement>);
pub fn fetch_roads_by_area() -> FetchRoadsByAreaStmt {
    FetchRoadsByAreaStmt(
        "SELECT osm_id, class, category, oneway, max_speed, layer, reference, is_bridge, is_tunnel, ST_asewkb(geom::geometry) as geom FROM osm_roads WHERE st_intersects(geom, st_setsrid(st_geomfromewkb($1), 4326)::geography)",
        None,
    )
}
impl FetchRoadsByAreaStmt {
    pub async fn prepare<'a, C: GenericClient>(
        mut self,
        client: &'a C,
    ) -> Result<Self, tokio_postgres::Error> {
        self.1 = Some(client.prepare(self.0).await?);
        Ok(self)
    }
    pub fn bind<'c, 'a, 's, C: GenericClient, T1: crate::BytesSql>(
        &'s self,
        client: &'c C,
        bounds: &'a T1,
    ) -> FetchRoadsByAreaQuery<'c, 'a, 's, C, FetchRoadsByArea, 1> {
        FetchRoadsByAreaQuery {
            client,
            params: [bounds],
            query: self.0,
            cached: self.1.as_ref(),
            extractor: |
                row: &tokio_postgres::Row,
            | -> Result<FetchRoadsByAreaBorrowed, tokio_postgres::Error> {
                Ok(FetchRoadsByAreaBorrowed {
                    osm_id: row.try_get(0)?,
                    class: row.try_get(1)?,
                    category: row.try_get(2)?,
                    oneway: row.try_get(3)?,
                    max_speed: row.try_get(4)?,
                    layer: row.try_get(5)?,
                    reference: row.try_get(6)?,
                    is_bridge: row.try_get(7)?,
                    is_tunnel: row.try_get(8)?,
                    geom: row.try_get(9)?,
                })
            },
            mapper: |it| FetchRoadsByArea::from(it),
        }
    }
}
pub struct FetchRoadsByAreaAndCategoryStmt(&'static str, Option<tokio_postgres::Statement>);
pub fn fetch_roads_by_area_and_category() -> FetchRoadsByAreaAndCategoryStmt {
    FetchRoadsByAreaAndCategoryStmt(
        "SELECT osm_id, class, category, oneway, max_speed, layer, reference, is_bridge, is_tunnel, ST_asewkb(geom::geometry) as geom FROM osm_roads WHERE category = $1 AND st_intersects(geom, st_setsrid(st_geomfromewkb($2), 4326)::geography)",
        None,
    )
}
impl FetchRoadsByAreaAndCategoryStmt {
    pub async fn prepare<'a, C: GenericClient>(
        mut self,
        client: &'a C,
    ) -> Result<Self, tokio_postgres::Error> {
        self.1 = Some(client.prepare(self.0).await?);
        Ok(self)
    }
    pub fn bind<'c, 'a, 's, C: GenericClient, T1: crate::BytesSql>(
        &'s self,
        client: &'c C,
        category: &'a crate::types::RoadClassCategory,
        bounds: &'a T1,
    ) -> FetchRoadsByAreaAndCategoryQuery<'c, 'a, 's, C, FetchRoadsByAreaAndCategory, 2> {
        FetchRoadsByAreaAndCategoryQuery {
            client,
            params: [category, bounds],
            query: self.0,
            cached: self.1.as_ref(),
            extractor: |
                row: &tokio_postgres::Row,
            | -> Result<FetchRoadsByAreaAndCategoryBorrowed, tokio_postgres::Error> {
                Ok(FetchRoadsByAreaAndCategoryBorrowed {
                    osm_id: row.try_get(0)?,
                    class: row.try_get(1)?,
                    category: row.try_get(2)?,
                    oneway: row.try_get(3)?,
                    max_speed: row.try_get(4)?,
                    layer: row.try_get(5)?,
                    reference: row.try_get(6)?,
                    is_bridge: row.try_get(7)?,
                    is_tunnel: row.try_get(8)?,
                    geom: row.try_get(9)?,
                })
            },
            mapper: |it| FetchRoadsByAreaAndCategory::from(it),
        }
    }
}
impl<'c, 'a, 's, C: GenericClient, T1: crate::BytesSql>
    crate::client::async_::Params<
        'c,
        'a,
        's,
        FetchRoadsByAreaAndCategoryParams<T1>,
        FetchRoadsByAreaAndCategoryQuery<'c, 'a, 's, C, FetchRoadsByAreaAndCategory, 2>,
        C,
    > for FetchRoadsByAreaAndCategoryStmt
{
    fn params(
        &'s self,
        client: &'c C,
        params: &'a FetchRoadsByAreaAndCategoryParams<T1>,
    ) -> FetchRoadsByAreaAndCategoryQuery<'c, 'a, 's, C, FetchRoadsByAreaAndCategory, 2> {
        self.bind(client, &params.category, &params.bounds)
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
        max_speed: &'a Option<i32>,
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
        "INSERT INTO osm_roads ( osm_id, reference, class, category, oneway, max_speed, layer, is_bridge, is_tunnel, geom ) SELECT s.osm_id, s.reference, s.class, s.category, s.oneway, s.max_speed, s.layer, s.is_bridge, s.is_tunnel, st_setsrid(st_geomfromewkb(s.geom), 4326)::geography FROM tmp_upsert_roads_streaming s ON CONFLICT(osm_id) DO UPDATE SET (reference, class, category, oneway, max_speed, layer, is_bridge, is_tunnel, geom) = (excluded.reference, excluded.class, excluded.category, excluded.oneway, excluded.max_speed, excluded.layer, excluded.is_bridge, excluded.is_tunnel, excluded.geom)",
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
pub struct UpsertBuildingsStreamingStartStmt(&'static str, Option<tokio_postgres::Statement>);
pub fn upsert_buildings_streaming_start() -> UpsertBuildingsStreamingStartStmt {
    UpsertBuildingsStreamingStartStmt(
        "CREATE TEMP TABLE tmp_upsert_buildings_streaming AS SELECT * FROM osm_buildings LIMIT 0",
        None,
    )
}
impl UpsertBuildingsStreamingStartStmt {
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
pub struct UpsertBuildingsStreamingTransferStmt(&'static str, Option<tokio_postgres::Statement>);
pub fn upsert_buildings_streaming_transfer() -> UpsertBuildingsStreamingTransferStmt {
    UpsertBuildingsStreamingTransferStmt(
        "COPY tmp_upsert_buildings_streaming ( osm_id, kind, geom ) FROM stdin binary",
        None,
    )
}
impl UpsertBuildingsStreamingTransferStmt {
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
pub struct UpsertBuildingsStreamingCommitStmt(&'static str, Option<tokio_postgres::Statement>);
pub fn upsert_buildings_streaming_commit() -> UpsertBuildingsStreamingCommitStmt {
    UpsertBuildingsStreamingCommitStmt(
        "INSERT INTO osm_buildings ( osm_id, kind, geom ) SELECT s.osm_id, s.kind, st_setsrid(st_geomfromewkb(s.geom), 4326)::geography FROM tmp_upsert_buildings_streaming s ON CONFLICT(osm_id) DO UPDATE SET (kind, geom) = (excluded.kind, excluded.geom)",
        None,
    )
}
impl UpsertBuildingsStreamingCommitStmt {
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
pub struct UpsertBuildingsStreamingEndStmt(&'static str, Option<tokio_postgres::Statement>);
pub fn upsert_buildings_streaming_end() -> UpsertBuildingsStreamingEndStmt {
    UpsertBuildingsStreamingEndStmt("DROP TABLE tmp_upsert_buildings_streaming", None)
}
impl UpsertBuildingsStreamingEndStmt {
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
pub struct FetchBuildingsByAreaStmt(&'static str, Option<tokio_postgres::Statement>);
pub fn fetch_buildings_by_area() -> FetchBuildingsByAreaStmt {
    FetchBuildingsByAreaStmt(
        "SELECT osm_id, kind, ST_asewkb(geom::geometry) as geom FROM osm_buildings WHERE st_intersects(geom, st_setsrid(st_geomfromewkb($1), 4326)::geography)",
        None,
    )
}
impl FetchBuildingsByAreaStmt {
    pub async fn prepare<'a, C: GenericClient>(
        mut self,
        client: &'a C,
    ) -> Result<Self, tokio_postgres::Error> {
        self.1 = Some(client.prepare(self.0).await?);
        Ok(self)
    }
    pub fn bind<'c, 'a, 's, C: GenericClient, T1: crate::BytesSql>(
        &'s self,
        client: &'c C,
        bounds: &'a T1,
    ) -> FetchBuildingsByAreaQuery<'c, 'a, 's, C, FetchBuildingsByArea, 1> {
        FetchBuildingsByAreaQuery {
            client,
            params: [bounds],
            query: self.0,
            cached: self.1.as_ref(),
            extractor: |
                row: &tokio_postgres::Row,
            | -> Result<FetchBuildingsByAreaBorrowed, tokio_postgres::Error> {
                Ok(FetchBuildingsByAreaBorrowed {
                    osm_id: row.try_get(0)?,
                    kind: row.try_get(1)?,
                    geom: row.try_get(2)?,
                })
            },
            mapper: |it| FetchBuildingsByArea::from(it),
        }
    }
}
pub struct UpsertWatersStreamingStartStmt(&'static str, Option<tokio_postgres::Statement>);
pub fn upsert_waters_streaming_start() -> UpsertWatersStreamingStartStmt {
    UpsertWatersStreamingStartStmt(
        "CREATE TEMP TABLE tmp_upsert_waters_streaming AS SELECT * FROM osm_waters LIMIT 0",
        None,
    )
}
impl UpsertWatersStreamingStartStmt {
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
pub struct UpsertWatersStreamingTransferStmt(&'static str, Option<tokio_postgres::Statement>);
pub fn upsert_waters_streaming_transfer() -> UpsertWatersStreamingTransferStmt {
    UpsertWatersStreamingTransferStmt(
        "COPY tmp_upsert_waters_streaming ( osm_id, class, geom ) FROM stdin binary",
        None,
    )
}
impl UpsertWatersStreamingTransferStmt {
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
pub struct UpsertWatersStreamingCommitStmt(&'static str, Option<tokio_postgres::Statement>);
pub fn upsert_waters_streaming_commit() -> UpsertWatersStreamingCommitStmt {
    UpsertWatersStreamingCommitStmt(
        "INSERT INTO osm_waters ( osm_id, class, geom ) SELECT s.osm_id, s.class, st_setsrid(st_geomfromewkb(s.geom), 4326)::geography FROM tmp_upsert_waters_streaming s ON CONFLICT(osm_id) DO UPDATE SET (class, geom) = (excluded.class, excluded.geom)",
        None,
    )
}
impl UpsertWatersStreamingCommitStmt {
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
pub struct UpsertWatersStreamingEndStmt(&'static str, Option<tokio_postgres::Statement>);
pub fn upsert_waters_streaming_end() -> UpsertWatersStreamingEndStmt {
    UpsertWatersStreamingEndStmt("DROP TABLE tmp_upsert_waters_streaming", None)
}
impl UpsertWatersStreamingEndStmt {
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
pub struct FetchWatersByAreaStmt(&'static str, Option<tokio_postgres::Statement>);
pub fn fetch_waters_by_area() -> FetchWatersByAreaStmt {
    FetchWatersByAreaStmt(
        "SELECT osm_id, class, ST_asewkb(geom::geometry) as geom FROM osm_waters WHERE st_intersects(geom, st_setsrid(st_geomfromewkb($1), 4326)::geography)",
        None,
    )
}
impl FetchWatersByAreaStmt {
    pub async fn prepare<'a, C: GenericClient>(
        mut self,
        client: &'a C,
    ) -> Result<Self, tokio_postgres::Error> {
        self.1 = Some(client.prepare(self.0).await?);
        Ok(self)
    }
    pub fn bind<'c, 'a, 's, C: GenericClient, T1: crate::BytesSql>(
        &'s self,
        client: &'c C,
        bounds: &'a T1,
    ) -> FetchWatersByAreaQuery<'c, 'a, 's, C, FetchWatersByArea, 1> {
        FetchWatersByAreaQuery {
            client,
            params: [bounds],
            query: self.0,
            cached: self.1.as_ref(),
            extractor: |
                row: &tokio_postgres::Row,
            | -> Result<FetchWatersByAreaBorrowed, tokio_postgres::Error> {
                Ok(FetchWatersByAreaBorrowed {
                    osm_id: row.try_get(0)?,
                    class: row.try_get(1)?,
                    geom: row.try_get(2)?,
                })
            },
            mapper: |it| FetchWatersByArea::from(it),
        }
    }
}
