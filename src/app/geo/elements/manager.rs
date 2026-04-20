use crate::app::geo::map::MapViewContextRef;
use crate::app::geo::map::{MapViewContextQuery, MapViewTransform};
use crate::app::utils::async_requests::{Request, RequestKind, RequestState, RequestWithManager};
use crate::app::utils::big_space_ext::CommandsWithSpatial;
use crate::app::utils::debug::SoftExpect;
use crate::geo::coords::Projection2D;
use bevy::prelude::*;
use bevy_inspector_egui::egui::emath::OrderedFloat;
use big_space::grid::Grid;
use glam::{DVec2, UVec2, dvec2};
use itertools::Itertools;
use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::marker::PhantomData;
use utilities::glam_ext::bounding::{AxisAlignedBoundingBox2D, DAabb2};

pub trait ElementId {
    fn id(&self) -> u64;
}

pub struct ManagerPlugin<
    T: ElementId + Send + Sync + 'static,
    K: RequestKind<Key = Bounds, Value = Vec<T>> + Send + Sync + 'static,
> {
    marker_t: PhantomData<T>,
    marker_k: PhantomData<K>,
}

impl<T, K> ManagerPlugin<T, K>
where
    T: ElementId + Send + Sync + 'static,
    K: RequestKind<Key = Bounds, Value = Vec<T>> + Send + Sync + 'static,
{
    pub fn new() -> Self {
        Self {
            marker_t: PhantomData,
            marker_k: PhantomData,
        }
    }
}

impl<T, K> Plugin for ManagerPlugin<T, K>
where
    T: ElementId + Send + Sync + 'static + Reflect + TypePath,
    K: RequestKind<Key = Bounds, Value = Vec<T>> + Reflect + TypePath + Send + Sync + 'static,
{
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                (
                    update_manager_spawnable_tile_indices::<T, K>,
                    handle_request_init_and_despawn::<T, K>,
                    setup_map_view_tile::<T, K>,
                )
                    .chain(),
                handle_loaded_requests::<T, K>,
            ),
        );

        app.register_type::<MapViewElementsManager<T, K>>();
    }
}

pub type Bounds = [OrderedFloat<f64>; 4];

#[derive(Component, Reflect)]
#[require(MapViewContextRef)]
pub struct MapViewElementsManager<T, K: RequestKind<Key = Bounds, Value = Vec<T>>> {
    pub spawning_viewport_abs_perc_max: f64,
    pub request_manager_id: Entity,

    #[reflect(ignore)]
    element_setup: Option<Box<dyn Fn(&mut EntityCommands, &T) + Send + Sync + 'static>>,

    active_tile_indices: Option<ActiveTileData>,

    tiles: HashMap<UVec2, Entity>,

    #[reflect(ignore)]
    marker: PhantomData<K>,
}

#[derive(PartialEq, Reflect)]
struct ActiveTileData {
    start_index: UVec2,
    end_index: UVec2,

    tiles_bounds: DAabb2,
    tile_size: DVec2,
}

impl ActiveTileData {
    fn is_active_index(&self, index: UVec2) -> bool {
        self.start_index.cmple(index).all() && index.cmple(self.end_index).all()
    }
}

impl<T, K: RequestKind<Key = Bounds, Value = Vec<T>>> MapViewElementsManager<T, K> {
    pub fn new(
        spawning_viewport_abs_perc_max: f64,
        element_setup: Box<dyn Fn(&mut EntityCommands, &T) + Send + Sync + 'static>,
        request_manager_id: Entity,
    ) -> Self {
        Self {
            spawning_viewport_abs_perc_max,
            element_setup: Some(element_setup),
            request_manager_id,
            active_tile_indices: None,
            tiles: HashMap::new(),
            marker: PhantomData,
        }
    }
}

fn update_manager_spawnable_tile_indices<
    T: Send + Sync + 'static,
    K: RequestKind<Key = Bounds, Value = Vec<T>> + Send + Sync + 'static,
>(
    managers: Query<(Entity, &mut MapViewElementsManager<T, K>)>,
    view_ctx: MapViewContextQuery,
) {
    for (manager_id, mut manager) in managers {
        if let Some(ctx) = view_ctx.get(manager_id).soft_expect("") {
            if let Some(viewport_abs) = ctx.view.viewport_abs {
                let viewport_abs_perc =
                    viewport_abs.size() / ctx.map.projection.abs_bounds().size();

                if viewport_abs_perc.min_element() > manager.spawning_viewport_abs_perc_max {
                    manager.active_tile_indices = None;
                } else {
                    let tile_size = ctx.map.projection.abs_bounds().size()
                        * manager.spawning_viewport_abs_perc_max;

                    let start_index =
                        ((viewport_abs.min() - ctx.map.projection.abs_bounds().min()) / tile_size)
                            .floor()
                            .as_uvec2();

                    let end_index = ((viewport_abs.max() - ctx.map.projection.abs_bounds().min())
                        / tile_size)
                        .ceil()
                        .as_uvec2();

                    let active_tile_indices = Some(ActiveTileData {
                        start_index,
                        end_index,
                        tile_size,
                        tiles_bounds: DAabb2::new(
                            ctx.map.projection.abs_bounds().min()
                                + start_index.as_dvec2() * tile_size,
                            ctx.map.projection.abs_bounds().min()
                                + end_index.as_dvec2() * tile_size,
                        ),
                    });

                    manager.active_tile_indices = active_tile_indices;
                }
            } else {
                manager.active_tile_indices = None;
            }
        }
    }
}

fn handle_request_init_and_despawn<
    T: Send + Sync + 'static,
    K: RequestKind<Key = Bounds, Value = Vec<T>> + Send + Sync + 'static,
>(
    mut commands: Commands,
    managers: Query<(Entity, &mut MapViewElementsManager<T, K>)>,
    view_ctx: MapViewContextQuery,
) {
    for (manager_id, mut manager) in managers {
        if let Some(ActiveTileData {
            start_index,
            end_index,
            tile_size,
            tiles_bounds,
        }) = manager.active_tile_indices
        {
            for (tile_index, tile_id) in manager
                .tiles
                .extract_if(|tile_index, _| {
                    tile_index.cmplt(start_index).any() || tile_index.cmpgt(end_index).any()
                })
                .collect_vec()
                .into_iter()
            {
                info!("despawn {:?}", tile_index);
                commands.entity(tile_id).despawn();
            }

            if let Some(ctx) = view_ctx.get(manager_id).soft_expect("") {
                for tile_index in (start_index.x..=end_index.x)
                    .flat_map(|x| (start_index.y..=end_index.y).map(move |y| UVec2::new(x, y)))
                {
                    if let Entry::Vacant(entry) = manager.tiles.entry(tile_index) {
                        let tile_start =
                            tiles_bounds.min() + (tile_index - start_index).as_dvec2() * tile_size;
                        let tile_end = tile_start + tile_size;

                        let gcs_start = ctx.map.projection.abs_to_gcs(tile_start);
                        let gcs_end = ctx.map.projection.abs_to_gcs(tile_end);

                        let tile_id = commands
                            .spawn_spatial((
                                MapViewTile {
                                    manager_id,
                                    tile_index,
                                    gcs_bounds: DAabb2::new(gcs_start, gcs_end),
                                },
                                Grid::default(),
                            ))
                            .id();

                        commands.entity(ctx.view_id).add_child(tile_id);

                        info!("spawn {:?}", tile_index);
                        entry.insert(tile_id);
                    }
                }
            }
        } else {
            for (_, tile_id) in manager.tiles.drain() {
                commands.entity(tile_id).despawn();
            }
        }
    }
}

#[derive(Component)]
pub struct MapViewTile {
    pub manager_id: Entity,
    pub tile_index: UVec2,
    pub gcs_bounds: DAabb2,
}

fn setup_map_view_tile<
    T: Send + Sync + 'static,
    K: RequestKind<Key = Bounds, Value = Vec<T>> + Send + Sync + 'static,
>(
    mut commands: Commands,
    tiles: Query<(Entity, &MapViewTile), Added<MapViewTile>>,
    managers: Query<&MapViewElementsManager<T, K>>,
) {
    for (tile_id, tile) in tiles {
        if let Some(manager) = managers.get(tile.manager_id).ok().soft_expect("") {
            commands.entity(tile_id).insert((
                Request::<K>::new(
                    [
                        tile.gcs_bounds.min().x.into(),
                        tile.gcs_bounds.min().y.into(),
                        tile.gcs_bounds.max().x.into(),
                        tile.gcs_bounds.max().y.into(),
                    ],
                    0,
                ),
                RequestWithManager(manager.request_manager_id),
                MapViewTransform {
                    translation: dvec2(0.0, 0.0),
                },
            ));
        }
    }
}

fn handle_loaded_requests<
    T: ElementId + Send + Sync + 'static,
    K: RequestKind<Key = Bounds, Value = Vec<T>> + Send + Sync + 'static,
>(
    mut commands: Commands,
    changed_requests: Query<(Entity, &Request<K>, &MapViewTile), Changed<Request<K>>>,
    mut managers: Query<&mut MapViewElementsManager<T, K>>,
) {
    for (tile_id, req, tile) in changed_requests {
        if let RequestState::Completed(Ok(res)) = req.state() {
            info!("loaded {:?} {:?}", tile.tile_index, tile.gcs_bounds);
            if let Some(mut manager) = managers.get_mut(tile.manager_id).ok().soft_expect("") {
                for element in res {
                    let el_id = commands
                        .spawn_spatial((MapViewElement {
                            tile_index: tile.tile_index,
                            element_id: element.id(),
                        },))
                        .id();

                    commands.entity(tile_id).add_child(el_id);

                    if let Some(element_setup) = &mut manager.element_setup {
                        (element_setup)(&mut commands.entity(el_id), element);
                    }
                }
            }
        }
    }
}

#[derive(Component)]
pub struct MapViewElement {
    pub tile_index: UVec2,
    pub element_id: u64,
}
