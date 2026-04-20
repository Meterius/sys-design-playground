use crate::app::geo::map::MapViewContextRef;
use crate::app::geo::map::MapViewContextQuery;
use crate::app::utils::big_space_ext::CommandsWithSpatial;
use crate::app::utils::debug::SoftExpect;
use crate::geo::coords::Projection2D;
use bevy::prelude::*;
use bevy_vector_shapes::prelude::{RectPainter, ShapePainter};
use bevy_vector_shapes::shapes::ThicknessType;
use big_space::grid::Grid;
use glam::UVec2;
use itertools::Itertools;
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use bevy::prelude::TransformSystems::Propagate;
use utilities::glam_ext::bounding::{AxisAlignedBoundingBox2D, DAabb2};
use crate::app::common::settings::Settings;

pub trait ElementId {
    fn id(&self) -> u64;
}

pub struct ManagerPlugin {}

impl Plugin for ManagerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                (sync_grid_active_tiles, sync_grid_spawned_tiles).chain(),
            ),
        );

        app.add_systems(
            PostUpdate,
            debug_tile_draw.after(Propagate).run_if(Settings::in_debug_mode),
        );
    }
}

pub type LinearGridKey = UVec2;

#[derive(Clone, Reflect)]
pub struct LinearGrid {
    pub count: UVec2,
    // percentage tile must take up of viewport to become active
    pub min_tile_viewport_percentage: Vec2,
}

impl LinearGrid {
    fn tiles(
        &self,
        world: DAabb2,
        viewport: DAabb2,
    ) -> impl Iterator<Item = (LinearGridKey, TileInfo)> {
        let tile_size = world.size() / self.count.as_dvec2();
        let tile_viewport_percentage = tile_size / viewport.size();

        let active = tile_viewport_percentage.as_vec2().cmpge(self.min_tile_viewport_percentage).all();

        let start_index = ((viewport.min() - world.min()) / tile_size)
            .floor()
            .as_uvec2()
            .clamp(UVec2::ZERO, self.count - UVec2::ONE);

        let end_index = ((viewport.max() - world.min()) / tile_size)
            .ceil()
            .as_uvec2()
            .clamp(UVec2::ZERO, self.count - UVec2::ONE);

        active.then_some(start_index.x..=end_index.x)
            .into_iter()
            .flatten()
            .flat_map(move |x| {
            (start_index.y..=end_index.y).map(move |y| {
                let info = TileInfo {
                    key: UVec2::new(x, y),
                    bounds: DAabb2::new(
                        world.min() + tile_size * UVec2::new(x, y).as_dvec2(),
                        world.min() + tile_size * UVec2::new(x + 1, y + 1).as_dvec2(),
                    ),
                };
                (info.key, info)
            })
        })
    }
}

#[derive(Clone, Reflect)]
pub struct TileInfo {
    key: LinearGridKey,
    bounds: DAabb2,
}

#[derive(Component, Reflect)]
#[require(MapViewContextRef)]
pub struct MapViewGrid {
    pub grid: LinearGrid,

    active_tiles: HashMap<LinearGridKey, TileInfo>,
    spawned_tiles: HashMap<LinearGridKey, Entity>,
}

impl MapViewGrid {
    pub fn new(grid: LinearGrid) -> Self {
        Self {
            grid,
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
                    .tiles(ctx.map.projection.abs_bounds(), viewport_abs)
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
            commands.entity(tile_id).despawn();
        }

        if let Some(ctx) = view_ctx.get(grid_id).soft_expect("") {
            for tile_info in active_tiles.values() {
                if let Entry::Vacant(entry) = grid.spawned_tiles.entry(tile_info.key) {
                    let gcs_bounds = DAabb2::new(
                        ctx.map.projection.abs_to_gcs(tile_info.bounds.min()),
                        ctx.map.projection.abs_to_gcs(tile_info.bounds.max()),
                    );

                    let tile_id = commands
                        .spawn_spatial((
                            MapViewTile {
                                manager_id: grid_id,
                                info: tile_info.clone(),
                                gcs_bounds,
                            },
                            Grid::default(),
                        ))
                        .id();

                    commands.entity(ctx.view_id).add_child(tile_id);

                    entry.insert(tile_id);
                }
            }
        }
    }
}

#[derive(Component, Reflect)]
pub struct MapViewTile {
    pub manager_id: Entity,
    pub gcs_bounds: DAabb2,
    pub info: TileInfo,
}

fn debug_tile_draw(
    grids: Query<(Entity, &MapViewGrid)>,
    view_ctx: MapViewContextQuery,
    transforms: Query<&GlobalTransform>,
    mut painter: ShapePainter,
) {
    for (grid_id, grid) in grids {
        if let Some(ctx) = view_ctx.get(grid_id).soft_expect("") && let Some(view_tr) = transforms.get(ctx.view_id).ok().soft_expect("") {
            painter.thickness = 2.0;
            painter.thickness_type = ThicknessType::Pixels;
            painter.hollow = true;
            painter.color = Color::srgba(0.1, 0.1, 1.0, 0.6);

            for tile in grid.active_tiles.values() {
                let start = view_tr.transform_point(ctx.view.abs_to_local(tile.bounds.min()).as_vec2().extend(0.0));
                let end = view_tr.transform_point(ctx.view.abs_to_local(tile.bounds.max()).as_vec2().extend(0.0));

                painter.set_translation((start + end) / 2.);
                painter.rect((end - start).xy());
            }
        }
    }
}
