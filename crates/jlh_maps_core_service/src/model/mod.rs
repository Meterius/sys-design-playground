use sea_orm::prelude::Json;
use sea_orm::{DerivePartialModel, FromQueryResult};
use serde::Serialize;

pub mod postgres_osm;

#[derive(DerivePartialModel, FromQueryResult, Serialize, Debug)]
#[sea_orm(entity = "postgres_osm::unitable::Entity")]
pub struct UnitablePartial {
    pub osm_type: String,
    pub osm_key: i64,
    pub attrs: Option<Json>,
    pub tags: Option<Json>,
}
