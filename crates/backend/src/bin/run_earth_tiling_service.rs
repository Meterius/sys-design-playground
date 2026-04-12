use backend::earth_tiling_service;

#[actix_web::main]
pub async fn main() -> std::io::Result<()> {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .compact()
        .init();

    earth_tiling_service::main::main().await
}
