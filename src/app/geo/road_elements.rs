use crate::app::geo::geometry::MapLine;
use crate::app::geo::grid::manager::{LinearGrid, MapViewGrid};
use crate::app::geo::zoom_visibility::MapZoomVisibility;
use crate::app::utils::async_requests::{
    AsyncRequestsPlugin, Request, RequestClient, RequestKind, RequestManager, RequestState,
    RequestWithManager,
};
use crate::app::utils::big_space_ext::CommandsWithSpatial;
use crate::app::utils::debug::SoftExpect;
use crate::geo::osm::client::{OsmClient, OsmError};
use crate::geo::osm::layered::model::road::{Road, RoadClass, RoadClassCategory};
use bevy::app::{App, Plugin};
use bevy::camera::visibility::RenderLayers;
use bevy::color::Color;
use bevy::prelude::*;
use bevy::tasks::futures_lite::StreamExt;
use big_space::grid::Grid;
use bimap::BiMap;
use geo_types::LineString;
use glam::{dvec2, uvec2, vec2, vec3};
use itertools::Itertools;
use ordered_float::OrderedFloat;
use ratelimit::Ratelimiter;
use rstar::primitives::GeomWithData;
use rstar::{AABB, RTree, RTreeObject};
use std::collections::HashMap;
use std::sync::Arc;
use tiff::encoder::TiffValue;
use utilities::glam_ext::bounding::{AxisAlignedBoundingBox2D, DAabb2};

pub struct RoadElementsPlugin {}

impl Plugin for RoadElementsPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((AsyncRequestsPlugin::<RoadRequestKind, RoadElementsClient>::new(),));
        app.register_type::<Request<RoadRequestKind>>();
        app.add_systems(
            Update,
            (
                on_road_tile_request_completed,
                on_road_inst_despawn_sync_provider,
                on_dirty_provider_spawns_missing_roads,
            )
                .chain(),
        );
    }
}

#[derive(Component, Reflect)]
pub struct RoadProvider {
    #[reflect(ignore)]
    roads: HashMap<i64, Road>,
    #[reflect(ignore)]
    roads_spatial: RTree<GeomWithData<LineString, i64>>,
    #[reflect(ignore)]
    spawned_roads: BiMap<i64, Entity>,
    dirty: bool,
}

fn on_road_tile_request_completed(
    tiles: Query<
        (&Request<RoadRequestKind>, &RoadTileWithProvider),
        Changed<Request<RoadRequestKind>>,
    >,
    mut providers: Query<&mut RoadProvider>,
) {
    for (request, RoadTileWithProvider(provider_id)) in tiles.iter() {
        if let RequestState::Completed(roads) = request.state()
            && let Ok(roads) = roads
            && let Some(mut provider) = providers.get_mut(*provider_id).ok().soft_expect("")
        {
            let new_roads = roads
                .iter()
                .filter(|r| !provider.roads.contains_key(&r.osm_id))
                .map(|r| {
                    GeomWithData::new(
                        LineString::from_iter(r.geometry.iter().map(|p| p.map(f64::to_radians).to_array())),
                        r.osm_id,
                    )
                })
                .collect_vec();

            for r in new_roads.into_iter() {
                provider.roads_spatial.insert(r);
            }

            provider
                .roads
                .extend(roads.iter().cloned().map(|road| (road.osm_id, road)));

            provider.dirty = true;
        }
    }
}

#[derive(Component, Reflect)]
#[relationship_target(relationship = RoadTileWithProvider)]
pub struct RoadProviderWithTiles(Vec<Entity>);

#[derive(Component, Reflect)]
#[relationship(relationship_target = RoadProviderWithTiles)]
pub struct RoadTileWithProvider(pub Entity);

#[derive(Component, Reflect)]
pub struct RoadTile {
    pub bounds_gcs: DAabb2,
    #[reflect(ignore)]
    pub road_filter: Option<Box<dyn Fn(&Road) -> bool + Send + Sync>>,
}

#[derive(Component, Reflect)]
#[relationship_target(relationship = RoadInstWithTile)]
pub struct RoadTileWithInsts(Vec<Entity>);

#[derive(Component, Reflect)]
#[relationship(relationship_target = RoadTileWithInsts)]
pub struct RoadInstWithTile(pub Entity);

#[derive(Component, Reflect)]
pub struct RoadInst;

fn make_road_bundle(road: &Road) -> impl Bundle {
    (
        Transform::from_translation(vec3(0.0, 0.0, 1000.0)),
        Name::new("Road"),
        Visibility::Visible,
        // MapZoomVisibility {
        //     visible_abs_view_perc: (
        //         0.0,
        //         0.00075
        //             * match road.class.category() {
        //                 RoadClassCategory::HighwayLinks => 3.0,
        //                 RoadClassCategory::MajorRoads => 3.0,
        //                 RoadClassCategory::MinorRoads => 0.85,
        //                 RoadClassCategory::Unknown => 0.25,
        //                 RoadClassCategory::VerySmallRoads => 0.25,
        //                 RoadClassCategory::PathsUnsuitableForCars => 0.25,
        //             },
        //     ),
        // },
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
    )
}

fn on_dirty_provider_spawns_missing_roads(
    mut commands: Commands,
    mut providers: Query<(&mut RoadProvider, &RoadProviderWithTiles)>,
    tiles: Query<&RoadTile>,
) {
    for (mut provider, RoadProviderWithTiles(provider_tiles)) in providers.iter_mut() {
        if provider.dirty {
            let mut spawned_roads = BiMap::new();
            std::mem::swap(&mut provider.spawned_roads, &mut spawned_roads);

            for &tile_id in provider_tiles {
                if let Some(tile) = tiles.get(tile_id).ok().soft_expect("") {
                    let eligible_roads = provider
                        .roads_spatial
                        .locate_in_envelope_intersecting(&AABB::from_corners(
                            tile.bounds_gcs.min().to_array().into(),
                            tile.bounds_gcs.max().to_array().into(),
                        ))
                        .filter(|r| !spawned_roads.contains_left(&r.data))
                        .flat_map(|r| provider.roads.get(&r.data))
                        .filter(|r| tile.road_filter.as_ref().is_some_and(|f| f(r)))
                        .collect_vec();

                    for road in eligible_roads.into_iter() {
                        let road_inst_id = commands
                            .spawn_spatial((
                                RoadInst,
                                RoadInstWithTile(tile_id),
                                make_road_bundle(road),
                            ))
                            .id();
                        commands.entity(tile_id).add_child(road_inst_id);
                        spawned_roads.insert(road.osm_id, road_inst_id);
                    }
                }
            }

            info!(
                "Spawned {} roads with {} total roads",
                spawned_roads.len(),
                provider.roads.len()
            );
            provider.spawned_roads = spawned_roads;
            provider.dirty = false;
        }
    }
}

fn on_road_inst_despawn_sync_provider(
    mut removed_road_insts: RemovedComponents<RoadInst>,
    mut providers: Query<&mut RoadProvider>,
) {
    for removed_inst_id in removed_road_insts.read() {
        for mut provider in providers.iter_mut() {
            if provider
                .spawned_roads
                .remove_by_right(&removed_inst_id)
                .is_some()
            {
                provider.dirty = true;
            }
        }
    }
}

pub fn spawn_roads_element_manager(
    commands: &mut Commands,
    view_id: Entity,
    client: Arc<OsmClient>,
) {
    let grid_id = commands
        .spawn_spatial((
            Grid::default(),
            Name::new("Road Grid"),
            RoadProvider {
                roads: HashMap::new(),
                spawned_roads: BiMap::new(),
                roads_spatial: RTree::new(),
                dirty: false,
            },
            RequestManager::new(
                10,
                Some(Ratelimiter::new(20)),
                RoadElementsClient { client },
            ),
        ))
        .id();

    commands.entity(grid_id).insert(MapViewGrid::new(
        None,
        LinearGrid {
            count: uvec2(1000, 1000),
            active_tile_buffer_using_expansion: uvec2(1, 1),
            active_tile_buffer_using_viewport_extension: vec2(0.2, 0.2),
            min_tile_viewport_percentage: vec2(0.25, 0.25),
        },
        Some(Box::new(move |commands, tile_id, tile| {
            commands.entity(tile_id).insert((
                RoadTile {
                    bounds_gcs: tile.bounds_gcs,
                    road_filter: Some(Box::new(|r| {
                        matches!(
                            r.class,
                            RoadClass::Primary | RoadClass::Motorway | RoadClass::MotorwayLink
                        )
                    })),
                },
                RoadTileWithProvider(grid_id),
                Request::<RoadRequestKind>::new(
                    [
                        tile.bounds_gcs.min().x.into(),
                        tile.bounds_gcs.min().y.into(),
                        tile.bounds_gcs.max().x.into(),
                        tile.bounds_gcs.max().y.into(),
                    ],
                    0,
                ),
                RequestWithManager(grid_id),
            ));

            // let medium_id = commands
            //     .spawn_spatial((
            //         Grid::default(),
            //         Name::new("Medium Road Grid"),
            //         MapViewGrid::new(
            //             Some(tile.bounds_abs),
            //             LinearGrid {
            //                 count: uvec2(4, 4),
            //                 active_tile_buffer_using_expansion: uvec2(1, 1),
            //                 active_tile_buffer_using_viewport_extension: vec2(0.2, 0.2),
            //                 min_tile_viewport_percentage: vec2(0.3, 0.3),
            //             },
            //             None,
            //         ),
            //     ))
            //     .id();
            // commands.entity(tile_id).add_child(medium_id);
            //
            // let small_id = commands
            //     .spawn_spatial((
            //         Grid::default(),
            //         Name::new("Small Road Grid"),
            //         MapViewGrid::new(
            //             Some(tile.bounds_abs),
            //             LinearGrid {
            //                 count: uvec2(12, 12),
            //                 active_tile_buffer_using_expansion: uvec2(1, 1),
            //                 active_tile_buffer_using_viewport_extension: vec2(0.1, 0.1),
            //                 min_tile_viewport_percentage: vec2(0.4, 0.4),
            //             },
            //             None,
            //         ),
            //     ))
            //     .id();
            //
            // commands.entity(tile_id).add_child(small_id);
        })),
    ));

    commands.entity(view_id).add_child(grid_id);
}

#[derive(Reflect)]
pub struct RoadRequestKind;

type Bounds = [OrderedFloat<f64>; 4];

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
        let roads: Vec<_> = self
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

        info!("Received {} roads", roads.len());

        Ok(roads)
    }
}
