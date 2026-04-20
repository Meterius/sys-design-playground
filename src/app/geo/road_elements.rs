use crate::app::geo::elements::ElementsPlugin;
use crate::app::geo::elements::manager::{Bounds, ElementId, MapViewElementsManager};
use crate::app::geo::geometry::MapLine;
use crate::app::geo::zoom_visibility::MapZoomVisibility;
use crate::app::utils::async_requests::{
    AsyncRequestsPlugin, RequestClient, RequestKind, RequestManager,
};
use crate::geo::osm::client::{OsmClient, OsmError};
use crate::geo::osm::layered::model::road::{Road, RoadClassCategory};
use bevy::app::{App, Plugin};
use bevy::camera::visibility::RenderLayers;
use bevy::color::Color;
use bevy::prelude::{Commands, Entity, Name, Reflect, Transform, Visibility};
use bevy::tasks::futures_lite::StreamExt;
use glam::{dvec2, vec3};
use itertools::Itertools;
use ratelimit::Ratelimiter;
use std::sync::Arc;
use utilities::glam_ext::bounding::{AxisAlignedBoundingBox2D, DAabb2};

pub struct RoadElementsPlugin {}

impl ElementId for Road {
    fn id(&self) -> u64 {
        self.osm_id.cast_unsigned()
    }
}

impl Plugin for RoadElementsPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            ElementsPlugin::<Road, RoadRequestKind>::new(),
            AsyncRequestsPlugin::<RoadRequestKind, RoadElementsClient>::new(),
        ));
    }
}

pub fn spawn_roads_element_manager(
    commands: &mut Commands,
    view_id: Entity,
    client: Arc<OsmClient>,
) {
    let req_client = RoadElementsClient { client };

    let req_manager_id = commands
        .spawn(RequestManager::new(
            10,
            Some(Ratelimiter::new(20)),
            req_client,
        ))
        .id();

    commands.entity(view_id).with_child((
        Name::new("RoadElementsManager"),
        MapViewElementsManager::<Road, RoadRequestKind>::new(
            0.005,
            Box::new(|commands, road| {
                commands.insert((
                    Transform::from_translation(vec3(0.0, 0.0, 1000.0)),
                    Name::new("Road"),
                    Visibility::Hidden,
                    MapZoomVisibility {
                        visible_abs_view_perc: (
                            0.0,
                            0.00075
                                * match road.class.category() {
                                    RoadClassCategory::HighwayLinks => 3.0,
                                    RoadClassCategory::MajorRoads => 3.0,
                                    RoadClassCategory::MinorRoads => 0.85,
                                    RoadClassCategory::Unknown => 0.25,
                                    RoadClassCategory::VerySmallRoads => 0.25,
                                    RoadClassCategory::PathsUnsuitableForCars => 0.25,
                                },
                        ),
                    },
                    MapLine::new(
                        road.geometry
                            .iter()
                            .map(|pos| dvec2(pos.x.to_radians(), pos.y.to_radians()))
                            .collect_vec(),
                        match road.class.category() {
                            RoadClassCategory::HighwayLinks => 6.0,
                            RoadClassCategory::MajorRoads => 3.0,
                            RoadClassCategory::MinorRoads => 1.0,
                            RoadClassCategory::Unknown => 0.1,
                            RoadClassCategory::VerySmallRoads => 0.2,
                            RoadClassCategory::PathsUnsuitableForCars => 0.25,
                        },
                        Color::hsva(38.0, 0.0, 0.7, 0.5),
                    ),
                    RenderLayers::layer(2),
                ));
            }),
            req_manager_id,
        ),
    ));
}

#[derive(Reflect)]
pub struct RoadRequestKind;

impl RequestKind for RoadRequestKind {
    type Key = Bounds;
    type Value = Vec<Road>;
    type Error = OsmError;
}

#[derive(Clone)]
pub struct RoadElementsClient {
    client: Arc<OsmClient>,
}

impl RequestClient<RoadRequestKind> for RoadElementsClient {
    async fn fetch_preflight(&self, _bounds: &Bounds) -> Result<Option<Vec<Road>>, OsmError> {
        Ok(None)
    }

    async fn fetch(&self, bounds: &Bounds) -> Result<Vec<Road>, OsmError> {
        let roads = self
            .client
            .fetch_roads_by_category(
                DAabb2::new(
                    dvec2(
                        bounds[0].into_inner().to_degrees(),
                        bounds[1].into_inner().to_degrees(),
                    ),
                    dvec2(
                        bounds[2].into_inner().to_degrees(),
                        bounds[3].into_inner().to_degrees(),
                    ),
                ),
                RoadClassCategory::MajorRoads,
            )
            .await?
            .try_collect()
            .await?;

        Ok(roads)
    }
}
