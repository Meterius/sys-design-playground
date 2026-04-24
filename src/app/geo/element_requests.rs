use crate::app::utils::async_requests::{AsyncRequestsPlugin, RequestClient, RequestKind};
use bevy::app::{App, Plugin};
use bevy::prelude::Reflect;
use bevy::tasks::futures_lite::StreamExt;
use glam::dvec2;
use ordered_float::OrderedFloat;
use osm::model::building::Building;
use osm::model::road::Road;
use osm::model::water::Water;
use osm::postgres_integration::client::{OsmClient, OsmError};
use std::sync::Arc;
use tracing::debug;
use utilities::glam_ext::bounding::{AxisAlignedBoundingBox2D, DAabb2};

pub struct ElementRequestsPlugin;

impl Plugin for ElementRequestsPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(AsyncRequestsPlugin::<RoadRequestKind, RoadRequestClient>::new());
        app.add_plugins(AsyncRequestsPlugin::<
            BuildingRequestKind,
            BuildingRequestClient,
        >::new());
        app.add_plugins(AsyncRequestsPlugin::<WaterRequestKind, WaterRequestClient>::new());
    }
}

pub type Bounds = [OrderedFloat<f64>; 4];

#[derive(Reflect)]
pub struct RoadRequestKind;

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

        debug!("Received {} roads", roads.len());

        Ok(roads)
    }
}

#[derive(Reflect)]
pub struct BuildingRequestKind;

impl RequestKind for BuildingRequestKind {
    type Key = Bounds;
    type Value = Vec<Building>;
    type Error = OsmError;
}

#[derive(Clone)]
pub struct BuildingRequestClient {
    pub client: Arc<OsmClient>,
}

impl RequestClient<BuildingRequestKind> for BuildingRequestClient {
    async fn fetch_preflight(
        &self,
        _bounds: &Bounds,
    ) -> bevy::prelude::Result<Option<Vec<Building>>, OsmError> {
        Ok(None)
    }

    async fn fetch(&self, bounds: &Bounds) -> bevy::prelude::Result<Vec<Building>, OsmError> {
        let buildings: Vec<_> = self
            .client
            .fetch_buildings(DAabb2::new(
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

        debug!("Received {} buildings", buildings.len());

        Ok(buildings)
    }
}

#[derive(Reflect)]
pub struct WaterRequestKind;

impl RequestKind for WaterRequestKind {
    type Key = Bounds;
    type Value = Vec<Water>;
    type Error = OsmError;
}

#[derive(Clone)]
pub struct WaterRequestClient {
    pub client: Arc<OsmClient>,
}

impl RequestClient<WaterRequestKind> for WaterRequestClient {
    async fn fetch_preflight(
        &self,
        _bounds: &Bounds,
    ) -> bevy::prelude::Result<Option<Vec<Water>>, OsmError> {
        Ok(None)
    }

    async fn fetch(&self, bounds: &Bounds) -> bevy::prelude::Result<Vec<Water>, OsmError> {
        let waters: Vec<_> = self
            .client
            .fetch_waters(DAabb2::new(
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

        debug!("Received {} waters", waters.len());

        Ok(waters)
    }
}
