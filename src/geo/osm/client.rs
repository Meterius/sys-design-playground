use geojson::FeatureCollection;
use thiserror::Error;

const GEO_FABRIK_INDEX: &str = "https://download.geofabrik.de/index-v1.json";

#[derive(Debug, Error)]
pub enum OsmError {
    #[error("Request Error: {0}")]
    ReqwestError(#[from] reqwest::Error),
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
