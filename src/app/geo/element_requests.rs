use crate::app::utils::async_requests::{AsyncRequestsPlugin, RequestClient, RequestKind};
use bevy::app::{App, Plugin};
use bevy::prelude::Reflect;
use bevy::tasks::futures_lite::StreamExt;
use glam::dvec2;
use ordered_float::OrderedFloat;
use osm::model::road::Road;
use osm::postgres_integration::client::{OsmClient, OsmError};
use std::sync::Arc;
use tracing::info;
use utilities::glam_ext::bounding::{AxisAlignedBoundingBox2D, DAabb2};

pub struct ElementRequestsPlugin;

impl Plugin for ElementRequestsPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(AsyncRequestsPlugin::<RoadRequestKind, RoadRequestClient>::new());
    }
}

#[derive(Reflect)]
pub struct RoadRequestKind;

pub type Bounds = [OrderedFloat<f64>; 4];

impl RequestKind for RoadRequestKind {
    type Key = Bounds;
    type Value = Vec<Road>;
    type Error = OsmError;
}

#[derive(Clone)]
pub struct RoadRequestClient {
    pub client: Arc<OsmClient>,
}

impl RequestClient<RoadRequestKind> for RoadRequestClient {
    async fn fetch_preflight(
        &self,
        _bounds: &Bounds,
    ) -> bevy::prelude::Result<Option<Vec<Road>>, OsmError> {
        Ok(None)
    }

    async fn fetch(&self, bounds: &Bounds) -> bevy::prelude::Result<Vec<Road>, OsmError> {
        let roads: Vec<_> = self
            .client
            .fetch_roads(DAabb2::new(
                dvec2(
                    bounds[0].into_inner().to_degrees(),
                    bounds[1].into_inner().to_degrees(),
                ),
                dvec2(
                    bounds[2].into_inner().to_degrees(),
                    bounds[3].into_inner().to_degrees(),
                ),
            ))
            .await?
            .try_collect()
            .await?;

        info!("Received {} roads", roads.len());

        Ok(roads)
    }
}
