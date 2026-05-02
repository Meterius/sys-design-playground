use std::io;
use strum::{Display, EnumString};

use actix_cors::Cors;
use actix_web::error::{ErrorInternalServerError, ErrorNotFound};
use actix_web::{
    App, Error, HttpServer, get, middleware,
    web::{self},
};
use jlh_maps_core_service::model::UnitablePartial;
use jlh_maps_core_service::model::postgres_osm::prelude::*;
use jlh_maps_core_service::model::postgres_osm::unitable;
use log::error;
use sea_orm::{Database, DatabaseConnection, EntityTrait};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
struct AppState {
    postgres_osm_conn: DatabaseConnection,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Serialize, Deserialize, Display, EnumString)]
#[serde(rename_all = "snake_case")]
pub enum OsmType {
    Node,
    Way,
    Relation,
}

#[get("/osm/element/{osm_type}/{osm_key}")]
async fn get_osm_element(
    data: web::Data<AppState>,
    path: web::Path<(OsmType, i64)>,
) -> Result<web::Json<UnitablePartial>, Error> {
    let (osm_type, osm_key) = path.into_inner();

    let item = Unitable::find_by_id((
        match osm_type {
            OsmType::Node => "N".into(),
            OsmType::Way => "W".into(),
            OsmType::Relation => "R".into(),
        },
        osm_key,
    ))
    .into_partial_model::<UnitablePartial>()
    .one(&data.postgres_osm_conn)
    .await
    .inspect_err(|err| error!("Error fetching unitable: {:?}", err))
    .map_err(ErrorInternalServerError)?;

    println!("{:?}", item);

    item.map(web::Json)
        .ok_or(ErrorNotFound("Element not found"))
}

#[actix_web::main]
async fn main() -> io::Result<()> {
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));

    dotenvy::dotenv().ok();

    let host = std::env::var("HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let port = std::env::var("PORT")
        .unwrap_or_else(|_| "8080".to_string())
        .parse::<u16>()
        .unwrap();
    let postgres_osm_url = std::env::var("POSTGRES_OSM_URL")
        .expect("POSTGRES_OSM_URL environment variable must be set");

    let postgres_osm_conn = Database::connect(&postgres_osm_url).await.unwrap();

    let app_state = AppState { postgres_osm_conn };

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(app_state.clone()))
            .service(get_osm_element)
            .wrap(Cors::permissive())
            .wrap(middleware::Logger::default())
    })
    .bind((host, port))?
    .run()
    .await
}
