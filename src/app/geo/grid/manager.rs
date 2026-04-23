use crate::app::common::settings::Settings;
use crate::app::geo::map::MapViewContextRef;
use crate::app::geo::map::{MapViewContext, MapViewContextQuery};
use crate::app::utils::big_space_ext::CommandsWithSpatial;
use crate::app::utils::debug::SoftExpect;
use crate::geo::coords::Projection2D;
use bevy::prelude::TransformSystems::Propagate;
use bevy::prelude::*;
use big_space::grid::Grid;
use glam::{DVec2, UVec2};
use itertools::Itertools;
use std::collections::HashMap;
use std::collections::hash_map::Entry;
use utilities::glam_ext::bounding::{AxisAlignedBoundingBox2D, DAabb2};
use crate::app::geo::despawn_indicator::DespawnIndicator;

#[derive(SystemSet, Clone, Debug, Eq, PartialEq, Hash)]
pub struct TileSpawningSystems;

pub struct ManagerPlugin {}

impl Plugin for ManagerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            ((
                sync_grid_active_tiles,
                sync_grid_spawned_tiles.in_set(TileSpawningSystems),
                handle_spawned_tiles,
            )
                .chain(),),
        );

        app.add_systems(
            PostUpdate,
            draw_debug_tile
                .after(Propagate)
                .run_if(Settings::in_debug_mode),
        );
    }
}

pub type LinearGridKey = UVec2;

#[derive(Clone, Reflect)]
pub struct LinearGrid {
    pub count: UVec2,
    // additional rows / columns of tiles to determine active beyond those in viewport
    pub active_tile_buffer_using_expansion: UVec2,
    // additional tiles to determine active using scaled viewport
    pub active_tile_buffer_using_viewport_extension: Vec2,
    // percentage tile must take up of viewport to become active
    pub min_tile_viewport_percentage: Vec2,
}

impl LinearGrid {
    fn unclamped_tile_bounds_covering_region(
        &self,
        world: DAabb2,
        region: DAabb2,
    ) -> (IVec2, IVec2) {
        (
            self.unclamped_pos_to_tile(world, region.min()),
            self.unclamped_pos_to_tile(world, region.max()),
        )
    }

    fn unclamped_pos_to_tile(&self, world: DAabb2, pos: DVec2) -> IVec2 {
        let tile_size = world.size() / self.count.as_dvec2();

        ((pos - world.min()) / tile_size).floor().as_ivec2()
    }

    pub fn pos_to_tile(&self, world: DAabb2, pos: DVec2) -> Option<UVec2> {
        let tile_idx = self.unclamped_pos_to_tile(world, pos);

        if IVec2::ZERO.cmple(tile_idx).all() && tile_idx.cmplt(self.count.as_ivec2()).all() {
            Some(tile_idx.as_uvec2())
        } else {
            None
        }
    }

    fn tiles(&self, world: DAabb2, viewport: DAabb2) -> impl Iterator<Item = (UVec2, DAabb2)> {
        let tile_size = world.size() / self.count.as_dvec2();

        let tile_viewport_percentage = tile_size / viewport.size();

        let active = tile_viewport_percentage
            .as_vec2()
            .cmpge(self.min_tile_viewport_percentage)
            .all();

        let (unbuffered_start_index, unbuffered_end_index) =
            self.unclamped_tile_bounds_covering_region(world, viewport);

        let expanded_viewport = viewport
            .expand(viewport.size() * self.active_tile_buffer_using_viewport_extension.as_dvec2());

        let (start_index, end_index) =
            self.unclamped_tile_bounds_covering_region(world, expanded_viewport);
        let (start_index, end_index) = (
            start_index
                .min(unbuffered_start_index - self.active_tile_buffer_using_expansion.as_ivec2()),
            end_index
                .max(unbuffered_end_index + self.active_tile_buffer_using_expansion.as_ivec2()),
        );

        let active = active && !end_index.cmplt(start_index).any();

        let start_index = start_index
            .clamp(IVec2::ZERO, self.count.as_ivec2() - IVec2::ONE)
            .as_uvec2();
        let end_index = end_index
            .clamp(IVec2::ZERO, self.count.as_ivec2() - IVec2::ONE)
            .as_uvec2();

        active
            .then_some(start_index.x..=end_index.x)
            .into_iter()
            .flatten()
            .flat_map(move |x| {
                (start_index.y..=end_index.y).map(move |y| {
                    (
                        uvec2(x, y),
                        DAabb2::new(
                            world.min() + tile_size * UVec2::new(x, y).as_dvec2(),
                            world.min() + tile_size * UVec2::new(x + 1, y + 1).as_dvec2(),
                        ),
                    )
                })
            })
    }
}

#[derive(Component, Reflect)]
#[require(MapViewContextRef)]
pub struct MapViewGrid {
    pub grid: LinearGrid,
    pub bounds_abs: Option<DAabb2>,
    #[reflect(ignore)]
    pub on_spawn:
        Option<Box<dyn Fn(&mut Commands, MapViewContext, Entity, &MapViewTile) + Send + Sync>>,

    active_tiles: HashMap<LinearGridKey, DAabb2>,
    spawned_tiles: HashMap<LinearGridKey, Entity>,
}

impl MapViewGrid {
    pub fn new(
        bounds_abs: Option<DAabb2>,
        grid: LinearGrid,
        on_spawn: Option<
            Box<dyn Fn(&mut Commands, MapViewContext, Entity, &MapViewTile) + Send + Sync>,
        >,
    ) -> Self {
        Self {
            bounds_abs,
            grid,
            on_spawn,
            active_tiles: HashMap::new(),
            spawned_tiles: HashMap::new(),
        }
    }
}

fn sync_grid_active_tiles(grids: Query<(Entity, &mut MapViewGrid)>, view_ctx: MapViewContextQuery) {
    for (grid_id, mut grid) in grids {
        if let Some(ctx) = view_ctx.get(grid_id).soft_expect("") {
            if let Some(viewport_abs) = ctx.view.viewport_abs {
                grid.active_tiles = grid
                    .grid
                    .tiles(
                        grid.bounds_abs
                            .unwrap_or_else(|| ctx.map.projection.abs_bounds()),
                        viewport_abs,
                    )
                    .collect();
            } else {
                grid.active_tiles.clear();
            }
        }
    }
}

fn sync_grid_spawned_tiles(
    mut commands: Commands,
    grids: Query<(Entity, &mut MapViewGrid)>,
    mut tiles: Query<&mut DespawnIndicator, With<MapViewTile>>,
    view_ctx: MapViewContextQuery,
) {
    for (grid_id, mut grid) in grids {
        let active_tiles = grid.active_tiles.clone();

        for (_, tile_id) in grid
            .spawned_tiles
            .extract_if(|key, _| !active_tiles.contains_key(key))
            .collect_vec()
            .into_iter()
        {
            if let Some(mut ind) = tiles.get_mut(tile_id).ok().soft_expect("") {
                *ind = DespawnIndicator::Despawning;
            }

            commands.entity(tile_id).despawn();
        }

        if let Some(ctx) = view_ctx.get(grid_id).soft_expect("") {
            for (key, bounds_abs) in active_tiles {
                if let Entry::Vacant(entry) = grid.spawned_tiles.entry(key) {
                    let bounds_gcs = DAabb2::new(
                        ctx.map.projection.abs_to_gcs(bounds_abs.min()),
                        ctx.map.projection.abs_to_gcs(bounds_abs.max()),
                    );

                    let tile_id = commands
                        .spawn_spatial((
                            DespawnIndicator::Active,
                            MapViewTile {
                                grid_id,
                                tile_idx: key,
                                bounds_abs,
                                bounds_gcs,
                            },
                            Grid::default(),
                        ))
                        .id();

                    commands.entity(grid_id).add_child(tile_id);

                    entry.insert(tile_id);
                }
            }
        }
    }
}

fn handle_spawned_tiles(
    mut commands: Commands,
    grids: Query<&MapViewGrid>,
    added_tiles: Query<(Entity, &MapViewTile), Added<MapViewTile>>,
    view_ctx: MapViewContextQuery,
) {
    for (tile_id, tile) in added_tiles {
        if let Some(grid) = grids.get(tile.grid_id).ok().soft_expect("")
            && let Some(ctx) = view_ctx.get(tile.grid_id).soft_expect("")
            && let Some(on_spawn) = grid.on_spawn.as_ref()
        {
            on_spawn(&mut commands, ctx, tile_id, tile);
        }
    }
}

#[derive(Component, Reflect)]
pub struct MapViewTile {
    pub grid_id: Entity,
    pub tile_idx: LinearGridKey,
    pub bounds_abs: DAabb2,
    pub bounds_gcs: DAabb2,
}

fn draw_debug_tile(
    grids: Query<(Entity, &MapViewGrid)>,
    view_ctx: MapViewContextQuery,
    transforms: Query<&GlobalTransform>,
    mut gizmos: Gizmos,
) {
    for (grid_id, grid) in grids {
        if let Some(ctx) = view_ctx.get(grid_id).soft_expect("")
            && let Some(view_tr) = transforms.get(ctx.view_id).ok().soft_expect("")
        {
            for (_, bounds_abs) in grid.active_tiles.iter() {
                let start = view_tr.transform_point(
                    ctx.view
                        .abs_to_local(bounds_abs.min())
                        .as_vec2()
                        .extend(0.0),
                );
                let end = view_tr.transform_point(
                    ctx.view
                        .abs_to_local(bounds_abs.max())
                        .as_vec2()
                        .extend(0.0),
                );

                gizmos
                    .rounded_rect(
                        Isometry3d::from_translation((start + end) / 2.),
                        (end - start).xy(),
                        Color::srgba(0.1, 0.1, 1.0, 0.6),
                    )
                    .corner_radius((end - start).xy().min_element() / 20.);
            }
        }
    }
}
