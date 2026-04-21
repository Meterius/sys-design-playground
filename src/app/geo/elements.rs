use crate::app::geo::geometry::MapLine;
use crate::app::geo::grid::manager::{LinearGrid, LinearGridKey, MapViewGrid};
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
use glam::{dvec2, uvec2, vec2, vec3, DVec2};
use itertools::Itertools;
use ordered_float::OrderedFloat;
use ratelimit::Ratelimiter;
use rstar::primitives::GeomWithData;
use rstar::{AABB, RTree, RTreeObject};
use std::collections::{HashMap, HashSet};
use std::hash::Hash;
use std::sync::Arc;
use reqwest::get;
use tiff::encoder::TiffValue;
use utilities::glam_ext::bounding::{AxisAlignedBoundingBox2D, DAabb2};
use crate::app::geo::map::{MapViewContextQuery, MapViewContextRef};
use crate::geo::coords::Projection2D;

pub struct ElementsPlugin {}

impl Plugin for ElementsPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((AsyncRequestsPlugin::<RoadRequestKind, RoadElementsClient>::new(),));
        app.register_type::<Request<RoadRequestKind>>();
        app.add_systems(
            Update,
            (on_request_completed, on_dirty_grid_tile_spawns_missing_roads).chain(),
        );
    }
}

pub trait Element {
    fn id(&self) -> i64;
}

impl Element for Road {
    fn id(&self) -> i64 {
        self.osm_id
    }
}

#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug, Reflect)]
pub enum RoadGridKind {
    Request,
    Large,
    Medium,
    Small
}

#[derive(Clone, Reflect)]
pub struct ElementTileGridConfig {
    grid: LinearGrid,
}

#[derive(Reflect)]
pub struct ElementsConfig<T: Element, G: Hash + Eq + Reflect>{
    request_grid: LinearGrid,
    tile_grids: HashMap<G, ElementTileGridConfig>,
    #[reflect(ignore)]
    get_tile_grid_for_element: Option<Box<dyn Fn(&T) -> Option<G> + Send + Sync>>,
}

#[derive(Component, Reflect)]
#[require(MapViewContextRef)]
pub struct ElementProvider<T: Element, G: Hash + Eq + Reflect> {
    config: ElementsConfig<T, G>,
    #[reflect(ignore)]
    grid_elements: HashMap<(G, LinearGridKey), HashMap<i64, T>>,
    #[reflect(ignore)]
    grid_elements_dirty: HashSet<(G, LinearGridKey)>,
}

fn on_request_completed(
    tiles: Query<
        (&Request<RoadRequestKind>, &RoadTileWithProvider),
        Changed<Request<RoadRequestKind>>,
    >,
    mut providers: Query<&mut ElementProvider<Road, RoadGridKind>>,
    view_ctx: MapViewContextQuery,
) {
    for (request, RoadTileWithProvider(provider_id)) in tiles.iter() {
        if let RequestState::Completed(roads) = request.state()
            && let Ok(roads) = roads
            && let Some(mut provider) = providers.get_mut(*provider_id).ok().soft_expect("")
        {
            if let Some(ctx) = view_ctx.get(*provider_id).soft_expect("") {
                let bounds_abs = ctx.map.projection.abs_bounds();

                let mut grid_elements = HashMap::new();
                std::mem::swap(&mut provider.grid_elements, &mut grid_elements);

                let mut grid_elements_dirty = HashSet::new();
                std::mem::swap(&mut provider.grid_elements_dirty, &mut grid_elements_dirty);

                if let Some(get_tile_grid_for_element) = &provider.config.get_tile_grid_for_element {
                    for road in roads.into_iter() {
                        if let Some(grid_idx) = get_tile_grid_for_element(&road) {
                            if let Some(grid_config) = provider.config.tile_grids.get(&grid_idx).soft_expect("") {
                                let geom = road.geometry.iter().map(|p| ctx.map.projection.gcs_to_abs(p.map(f64::to_radians)));
                                let pos = geom.sum::<DVec2>() / road.geometry.len() as f64;

                                if let Some(tile_idx) = grid_config.grid.pos_to_tile(bounds_abs, pos) {
                                    let elements = grid_elements.entry((grid_idx, tile_idx)).or_insert(HashMap::new());

                                    if elements.insert(road.id(), road.clone()).is_none() {
                                        grid_elements_dirty.insert((grid_idx, tile_idx));
                                    }
                                }
                            }
                        }
                    }
                }

                provider.grid_elements = grid_elements;
                provider.grid_elements_dirty = grid_elements_dirty;
            }
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
    pub grid_idx: RoadGridKind,
    pub tile_idx: LinearGridKey,
    pub spawned_roads: HashSet<i64>,
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

fn on_dirty_grid_tile_spawns_missing_roads(
    mut commands: Commands,
    mut providers: Query<&mut ElementProvider<Road, RoadGridKind>, Without<RoadTile>>,
    mut tiles: Query<(Entity, &mut RoadTile, &RoadTileWithProvider), Without<ElementProvider<Road, RoadGridKind>>>,
) {
    for (tile_id, mut tile, RoadTileWithProvider(provider_id)) in tiles.iter_mut() {
        if let Some(mut provider) = providers.get_mut(*provider_id).ok().soft_expect("") {
            if provider.grid_elements_dirty.remove(&(tile.grid_idx, tile.tile_idx)) || tile.is_added() {
                if let Some(roads) = provider.grid_elements.get(&(tile.grid_idx, tile.tile_idx)) {
                    for (&road_id, road) in roads {
                        if tile.spawned_roads.insert(road_id) {
                            let road_inst_id = commands
                                .spawn_spatial((
                                    RoadInst,
                                    RoadInstWithTile(tile_id),
                                    make_road_bundle(road),
                                ))
                                .id();
                            commands.entity(tile_id).add_child(road_inst_id);
                        }
                    }
                }
            }
        }
    }
}


pub fn spawn_roads_element_manager(
    commands: &mut Commands,
    view_id: Entity,
    client: Arc<OsmClient>,
) {
    let make_grid = |count: UVec2, max_spawned: UVec2| -> LinearGrid {
        LinearGrid {
            count,
            active_tile_buffer_using_expansion: uvec2(1, 1),
            active_tile_buffer_using_viewport_extension: vec2(0.2, 0.2),
            min_tile_viewport_percentage: Vec2::ONE / max_spawned.as_vec2(),
        }
    };

    let config = ElementsConfig::<Road, RoadGridKind> {
        request_grid: make_grid(uvec2(1000, 1000), uvec2(4, 4)),
        tile_grids: HashMap::from([
            (RoadGridKind::Large, ElementTileGridConfig {
                grid: make_grid(uvec2(1000, 1000), uvec2(2, 2)),
            }),
        ]),
        get_tile_grid_for_element: Some(Box::new(|r: &Road| match r.class {
            RoadClass::Primary | RoadClass::Motorway | RoadClass::MotorwayLink => Some(RoadGridKind::Large),
            _ => None,
        })),
    };

    let provider_id = commands
        .spawn_spatial((
            Grid::default(),
            Name::new("Roads"),
            RequestManager::new(
                10,
                Some(Ratelimiter::new(20)),
                RoadElementsClient { client },
            ),
        ))
        .id();

    let request_grid_id = commands.spawn_spatial((
        Grid::default(),
        Name::new("Request Grid"),
        MapViewGrid::new(
            None,
            config.request_grid.clone(),
            Some(Box::new(move |commands, tile_id, tile| {
                commands.entity(tile_id).insert((
                    RoadTile {
                        grid_idx: RoadGridKind::Request,
                        bounds_gcs: tile.bounds_gcs,
                        tile_idx: tile.tile_idx,
                        spawned_roads: HashSet::new(),
                    },
                    RoadTileWithProvider(provider_id),
                    Request::<RoadRequestKind>::new(
                        [
                            tile.bounds_gcs.min().x.into(),
                            tile.bounds_gcs.min().y.into(),
                            tile.bounds_gcs.max().x.into(),
                            tile.bounds_gcs.max().y.into(),
                        ],
                        0,
                    ),
                    RequestWithManager(provider_id),
                ));
            })),
        )
    )).id();
    commands.entity(provider_id).add_child(request_grid_id);

    for (&grid_idx, grid_config) in config.tile_grids.iter() {
        let grid_id = commands.spawn_spatial((
            Grid::default(),
            Name::new(format!("Road Grid {grid_idx:?}")),
            MapViewGrid::new(
                None,
                grid_config.grid.clone(),
                Some(Box::new(move |commands, tile_id, tile| {
                    commands.entity(tile_id).insert((
                        RoadTile {
                            grid_idx,
                            bounds_gcs: tile.bounds_gcs,
                            tile_idx: tile.tile_idx,
                            spawned_roads: HashSet::new(),
                        },
                        RoadTileWithProvider(provider_id),
                    ));
                })),
            )
        )).id();
        commands.entity(provider_id).add_child(grid_id);
    }

    commands.entity(provider_id).insert(
        ElementProvider {
            config,
            grid_elements: HashMap::new(),
            grid_elements_dirty: HashSet::new(),
        },
    );
    commands.entity(view_id).add_child(provider_id);
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
