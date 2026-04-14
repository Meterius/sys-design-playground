use backend::earth_tiling_service;
use tracing_subscriber::EnvFilter;

#[actix_web::main]
pub async fn main() -> std::io::Result<()> {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_env_filter(EnvFilter::from_default_env())
        .compact()
        .init();

    earth_tiling_service::main::main().await
}
