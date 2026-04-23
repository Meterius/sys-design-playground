use crate::app::geo::despawn_indicator::DespawnIndicator;
use crate::app::geo::element_requests::Bounds;
use crate::app::geo::grid::manager::{LinearGrid, LinearGridKey, MapViewGrid, TileSpawningSystems};
use crate::app::geo::map::{MapViewContextQuery, MapViewContextRef};
use crate::app::utils::async_requests::{
    Request, RequestClient, RequestKind, RequestManager, RequestState, RequestWithManager,
};
use crate::app::utils::big_space_ext::CommandsWithSpatial;
use crate::app::utils::debug::SoftExpect;
use crate::geo::coords::Projection2D;
use bevy::app::{App, Plugin};
use bevy::camera::visibility::{NoFrustumCulling, RenderLayers};
use bevy::prelude::*;
use bevy_vello::prelude::VelloScene2d;
use big_space::grid::Grid;
use glam::DVec2;
use osm::model::road::Road;
use std::collections::{HashMap, HashSet};
use std::fmt::Debug;
use std::hash::Hash;
use std::marker::PhantomData;
use utilities::glam_ext::bounding::{AxisAlignedBoundingBox2D, DAabb2};

pub struct ElementsGridPlugin<RK, GK> {
    marker: PhantomData<(RK, GK)>,
}

impl<RK, GK> Default for ElementsGridPlugin<RK, GK> {
    fn default() -> Self {
        Self {
            marker: PhantomData,
        }
    }
}

impl<T, RK, GK> Plugin for ElementsGridPlugin<RK, GK>
where
    T: Element + Clone + Send + Sync + 'static,
    RK: RequestKind<Key = Bounds, Value = Vec<T>> + Reflect + Send + Sync + 'static,
    GK: Reflect + Debug + Copy + Eq + Hash + Send + Sync + 'static,
{
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                on_request_completed::<T, RK, GK>,
                on_dirty_grid_tile_spawns_missing_elements::<T, GK>.after(TileSpawningSystems),
            )
                .chain(),
        );
    }
}

// TODO: refactor into clean component-based lifecycle management
pub fn spawn_elements_grid<T, RK, GK, GC>(
    commands: &mut Commands,
    view_id: Entity,
    config: ElementsConfig<T, GK>,
    request_manager: RequestManager<RK, GC>,
) where
    T: Send + Sync + 'static,
    RK: RequestKind<Key = Bounds> + 'static,
    GK: Reflect + Debug + Copy,
    GC: RequestClient<RK> + Send + Sync + 'static,
{
    let provider_id = commands
        .spawn_spatial((Grid::default(), Name::new("Provider"), request_manager))
        .id();

    let request_grid_id = commands
        .spawn_spatial((
            Grid::default(),
            Name::new("Request Grid"),
            MapViewGrid::new(
                None,
                config.request_grid.clone(),
                Some(Box::new(move |commands, _, tile_id, tile| {
                    commands.entity(tile_id).insert((
                        ElementRequest { provider_id },
                        Request::<RK>::new(
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
            ),
        ))
        .id();
    commands.entity(provider_id).add_child(request_grid_id);

    for (&grid_idx, grid_config) in config.tile_grids.iter() {
        let grid_id = commands
            .spawn_spatial((
                Grid::default(),
                Name::new(format!("Grid {grid_idx:?}")),
                MapViewGrid::new(
                    None,
                    grid_config.grid.clone(),
                    Some(Box::new(move |commands, ctx, tile_id, tile| {
                        // TODO: use proper view grid to determine position, refactor hierarchy coordinate translations to allow for relative positioning
                        let center_local = ctx.view.abs_to_local(tile.bounds_abs.center());
                        let (cell_idx, cell_pos) =
                            Grid::default().translation_to_grid(center_local.extend(0.));

                        commands.entity(tile_id).insert((
                            cell_idx,
                            Transform::from_translation(cell_pos),
                            ElementTile {
                                grid_idx,
                                bounds_gcs: tile.bounds_gcs,
                                bounds_abs: tile.bounds_abs,
                                tile_idx: tile.tile_idx,
                                spawned_roads: HashSet::new(),
                            },
                            VelloScene2d::default(),
                            RenderLayers::layer(4),
                            NoFrustumCulling,
                            ElementTileWithProvider(provider_id),
                        ));
                    })),
                ),
            ))
            .id();
        commands.entity(provider_id).add_child(grid_id);
    }

    commands.entity(provider_id).insert(ElementProvider {
        config,
        grid_elements: HashMap::new(),
        grid_elements_dirty: HashSet::new(),
    });
    commands.entity(view_id).add_child(provider_id);
}

pub trait Element {
    fn id(&self) -> i64;
    fn aabb(&self) -> DAabb2;
}

impl Element for Road {
    fn id(&self) -> i64 {
        self.osm_id
    }

    fn aabb(&self) -> DAabb2 {
        DAabb2::new(
            self.aabb().min().map(f64::to_radians),
            self.aabb().max().map(f64::to_radians),
        )
    }
}

#[derive(Clone, Reflect)]
pub struct ElementTileGridConfig {
    pub grid: LinearGrid,
}

#[derive(Reflect)]
pub struct ElementsConfig<T, GK: Reflect> {
    pub request_grid: LinearGrid,
    pub tile_grids: HashMap<GK, ElementTileGridConfig>,
    #[reflect(ignore)]
    pub get_tile_grid_for_element: Option<Box<dyn Fn(&T) -> Option<GK> + Send + Sync>>,
    #[reflect(ignore)]
    pub on_spawn_element_instance:
        Option<Box<dyn Fn(&mut Commands, DVec2, Entity, Entity, &T) + Send + Sync>>,
}

#[derive(Component, Reflect)]
#[require(MapViewContextRef)]
pub struct ElementProvider<T, GK: Reflect> {
    config: ElementsConfig<T, GK>,
    #[reflect(ignore)]
    grid_elements: HashMap<(GK, LinearGridKey), HashMap<i64, T>>,
    #[reflect(ignore)]
    grid_elements_dirty: HashSet<(GK, LinearGridKey)>,
}

impl<T, GK: Reflect> ElementProvider<T, GK> {
    pub fn new(config: ElementsConfig<T, GK>) -> Self {
        Self {
            config,
            grid_elements: HashMap::new(),
            grid_elements_dirty: HashSet::new(),
        }
    }
}

fn on_request_completed<T, RK, GK>(
    tiles: Query<(&Request<RK>, &ElementRequest), Changed<Request<RK>>>,
    mut providers: Query<&mut ElementProvider<T, GK>>,
    view_ctx: MapViewContextQuery,
) where
    T: Element + Send + Sync + Clone + 'static,
    RK: RequestKind<Value = Vec<T>> + Reflect,
    GK: Eq + Hash + Copy + Reflect,
{
    for (request, request_p) in tiles.iter() {
        if let RequestState::Completed(roads) = request.state()
            && let Ok(roads) = roads
            && let Some(mut provider) = providers
                .get_mut(request_p.provider_id)
                .ok()
                .soft_expect("")
            && let Some(ctx) = view_ctx.get(request_p.provider_id).soft_expect("")
        {
            let bounds_abs = ctx.map.projection.abs_bounds();

            let mut grid_elements = HashMap::new();
            std::mem::swap(&mut provider.grid_elements, &mut grid_elements);

            let mut grid_elements_dirty = HashSet::new();
            std::mem::swap(&mut provider.grid_elements_dirty, &mut grid_elements_dirty);

            if let Some(get_tile_grid_for_element) = &provider.config.get_tile_grid_for_element {
                for el in roads.iter() {
                    if let Some(grid_idx) = get_tile_grid_for_element(el)
                        && let Some(grid_config) =
                            provider.config.tile_grids.get(&grid_idx).soft_expect("")
                    {
                        let center = (ctx.map.projection.gcs_to_abs(el.aabb().min())
                            + ctx.map.projection.gcs_to_abs(el.aabb().max()))
                            / 2.;

                        if let Some(tile_idx) = grid_config.grid.pos_to_tile(bounds_abs, center) {
                            let elements = grid_elements
                                .entry((grid_idx, tile_idx))
                                .or_insert(HashMap::new());

                            if elements.insert(el.id(), el.clone()).is_none() {
                                grid_elements_dirty.insert((grid_idx, tile_idx));
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

#[derive(Component, Reflect)]
pub struct ElementRequest {
    provider_id: Entity,
}

#[derive(Component, Reflect)]
#[relationship_target(relationship = ElementTileWithProvider)]
pub struct ElementProviderWithTiles(Vec<Entity>);

#[derive(Component, Reflect)]
#[relationship(relationship_target = ElementProviderWithTiles)]
pub struct ElementTileWithProvider(pub Entity);

#[derive(Component, Reflect)]
pub struct ElementTile<GK> {
    pub bounds_gcs: DAabb2,
    pub bounds_abs: DAabb2,
    pub grid_idx: GK,
    pub tile_idx: LinearGridKey,
    pub spawned_roads: HashSet<i64>,
}

fn on_dirty_grid_tile_spawns_missing_elements<T, GK>(
    mut commands: Commands,
    mut providers: Query<&mut ElementProvider<Road, GK>, Without<ElementTile<GK>>>,
    mut tiles: Query<
        (
            Entity,
            &mut ElementTile<GK>,
            &ElementTileWithProvider,
            &DespawnIndicator,
        ),
        Without<ElementProvider<T, GK>>,
    >,
) where
    T: Element + Send + Sync + Clone + 'static,
    GK: Copy + Eq + Hash + Reflect,
{
    for (tile_id, mut tile, ElementTileWithProvider(provider_id), &ind) in tiles.iter_mut() {
        if ind == DespawnIndicator::Active
            && let Some(mut provider) = providers.get_mut(*provider_id).ok().soft_expect("")
            && (provider
                .grid_elements_dirty
                .remove(&(tile.grid_idx, tile.tile_idx))
                || tile.is_added())
            && let Some(roads) = provider.grid_elements.get(&(tile.grid_idx, tile.tile_idx))
        {
            for (&road_id, road) in roads {
                if tile.spawned_roads.insert(road_id) {
                    let road_inst_id = commands.spawn_spatial(Visibility::Inherited).id();

                    if let Some(on_spawn_element_instance) =
                        provider.config.on_spawn_element_instance.as_ref()
                    {
                        on_spawn_element_instance(
                            &mut commands,
                            tile.bounds_abs.center(),
                            tile_id,
                            road_inst_id,
                            road,
                        );
                    }

                    commands.entity(tile_id).add_child(road_inst_id);
                }
            }
        }
    }
}
