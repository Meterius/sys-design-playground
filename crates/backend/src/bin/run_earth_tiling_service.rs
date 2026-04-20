use backend::earth_tiling_service;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::EnvFilter;

#[actix_web::main]
pub async fn main() -> std::io::Result<()> {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::builder()
                .with_default_directive(LevelFilter::INFO.into())
                .from_env_lossy(),
        )
        .compact()
        .init();

    earth_tiling_service::main::main().await
}
