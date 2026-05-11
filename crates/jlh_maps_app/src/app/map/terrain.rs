use crate::app::common::debug_gizmos::DebugAabbGizmo;
use crate::app::common::materials::TransparentOverwriteMaterial;
use crate::app::map::core::{MAP_VIEW_COLOR_RENDER_LAYER, MAP_VIEW_DEPTH_RENDER_LAYER};
use crate::app::map::transform::{MERCATOR_WORLD_SIZE, lng_lat_to_world};
use crate::app::maplibre_gl_js::integration::MaplibreMapIntegration;
use crate::app::maplibre_gl_js::types::CanonicalTileId;
use crate::app::maplibre_gl_js::utils::mercator_coordinate::{
    EARTH_CIRCUMFERENCE, LngLat, MercatorCoordinate, lat_from_mercator_y, lng_from_mercator_x,
};
use crate::app::maplibre_gl_js::utils::terrain::get_dem_elevation;
use crate::utils::debug::SoftExpect;
use crate::utils::terrain_mesh::build_terrain_mesh_with_skirts;
use bevy::camera::visibility::RenderLayers;
use bevy::math::{DVec2, DVec3, dvec2};
use bevy::prelude::*;
use big_space::grid::Grid;
use big_space::prelude::CellCoord;
use std::collections::HashMap;
use std::collections::hash_map::Entry;

const TILE_TERRAIN_MESH_RESOLUTION: u32 = 128;

pub struct TerrainPlugin;

impl Plugin for TerrainPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (sync_spawned_tiles, sync_tiles).chain());
    }
}

#[derive(Component)]
pub struct TerrainTileManager {
    pub maplibre_int_id: Entity,
    pub spawned_tiles: HashMap<CanonicalTileId, Entity>,
}

fn sync_spawned_tiles(
    mut commands: Commands,
    map_ints: Query<&MaplibreMapIntegration>,
    mut managers: Query<(Entity, &mut TerrainTileManager)>,
    mut materials: ResMut<Assets<TransparentOverwriteMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    for (manager_id, mut manager) in managers.iter_mut() {
        let maplibre_int_id = manager.maplibre_int_id;

        let Some(map_int) = map_ints.get(maplibre_int_id).ok().soft_expect("") else {
            continue;
        };

        for (_, tile_entity) in manager
            .spawned_tiles
            .extract_if(|tile_id, _| !map_int.terrain.active_tile_ids.contains(tile_id))
        {
            commands.entity(tile_entity).despawn();
        }

        for &tile_id in map_int.terrain.active_tile_ids.iter() {
            if let Entry::Vacant(entry) = manager.spawned_tiles.entry(tile_id) {
                let tile_e_id = commands
                    .spawn((
                        Name::new(format!("Terrain Tile {tile_id:?}")),
                        Transform::default(),
                        CellCoord::default(),
                        Mesh3d(meshes.add(Mesh::from(Plane3d::new(Vec3::Z, Vec2::ONE / 2.0)))),
                        MeshMaterial3d(materials.add(TransparentOverwriteMaterial::default())),
                        DebugAabbGizmo,
                        TerrainTile {
                            maplibre_int_id,
                            maplibre_tile_id: tile_id,
                            prev_terrain_hash: None,
                        },
                        RenderLayers::from_layers(&[
                            MAP_VIEW_DEPTH_RENDER_LAYER,
                            MAP_VIEW_COLOR_RENDER_LAYER,
                        ]),
                    ))
                    .id();
                commands.entity(manager_id).add_child(tile_e_id);
                entry.insert(tile_e_id);
            }
        }
    }
}

#[derive(Component)]
pub struct TerrainTile {
    pub maplibre_int_id: Entity,
    pub maplibre_tile_id: CanonicalTileId,
    pub prev_terrain_hash: Option<String>,
}

fn sync_tiles(
    map_ints: Query<&MaplibreMapIntegration>,
    mut tiles: Query<(
        &mut TerrainTile,
        &mut Transform,
        &mut CellCoord,
        &mut Mesh3d,
        &ChildOf,
    )>,
    grids: Query<&Grid>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    for (mut tile, mut tile_transform, mut tile_cell, mut tile_mesh, ChildOf(tile_parent_id)) in
        tiles.iter_mut()
    {
        let Some(map_int) = map_ints.get(tile.maplibre_int_id).ok().soft_expect("") else {
            continue;
        };
        let Some(grid) = grids.get(*tile_parent_id).ok().soft_expect("") else {
            continue;
        };

        let (tile_pos, tile_size) = tile_transform_d(tile.maplibre_tile_id, 0.);
        let (new_tile_cell, new_tile_cell_pos) = grid.translation_to_grid(tile_pos);
        let new_tile_cell_transform = Transform::from_translation(new_tile_cell_pos)
            .with_scale(tile_size.as_vec2().extend(1.0));

        *tile_transform = new_tile_cell_transform;
        *tile_cell = new_tile_cell;

        match map_int.terrain.tiles.get(&tile.maplibre_tile_id) {
            None if tile.prev_terrain_hash.is_some() => {
                tile_mesh.0 = meshes.add(Mesh::from(Plane3d::new(Vec3::Z, Vec2::ONE / 2.0)));
                tile.prev_terrain_hash = None;
            }
            Some(terrain_data) if Some(&terrain_data.hash) != tile.prev_terrain_hash.as_ref() => {
                let bounds = get_tile_lnglat_bounds(tile.maplibre_tile_id);

                let get_elevation = |uv: Vec2| {
                    let uv = vec2(0.0, 1.0) + vec2(1.0, -1.0) * uv;

                    let lnglat = bounds.0 + (bounds.1 - bounds.0) * uv.as_dvec2();

                    let dem_elev =
                        get_dem_elevation(&terrain_data.terrain_data, uv).unwrap_or(0.0) as f64;

                    (MercatorCoordinate::from_lng_lat(LngLat::new(lnglat.x, lnglat.y), dem_elev).z
                        * MERCATOR_WORLD_SIZE) as f32
                };

                let mesh_handle = meshes.add(build_terrain_mesh_with_skirts(
                    &get_elevation,
                    TILE_TERRAIN_MESH_RESOLUTION,
                    terrain_skirt_delta(tile.maplibre_tile_id),
                ));
                *tile_mesh = Mesh3d(mesh_handle);

                tile.prev_terrain_hash = Some(terrain_data.hash.clone());
            }
            _ => {}
        }
    }
}

fn get_tile_lnglat_bounds(id: CanonicalTileId) -> (DVec2, DVec2) {
    let ll_min = dvec2(
        lng_from_mercator_x(id.x as f64 / 2f64.powf(id.z as f64)),
        lat_from_mercator_y((id.y as f64 + 1.0) / 2f64.powf(id.z as f64)),
    );

    let ll_max = dvec2(
        lng_from_mercator_x((id.x + 1) as f64 / 2f64.powf(id.z as f64)),
        lat_from_mercator_y(id.y as f64 / 2f64.powf(id.z as f64)),
    );

    (ll_min.min(ll_max), ll_min.max(ll_max))
}

fn tile_transform_d(tile_id: CanonicalTileId, alt: f64) -> (DVec3, DVec2) {
    let bounds = get_tile_lnglat_bounds(tile_id);

    let south_west = lng_lat_to_world(bounds.0.x, bounds.0.y, alt);
    let north_east = lng_lat_to_world(bounds.1.x, bounds.1.y, alt);

    let min = south_west.min(north_east);
    let max = south_west.max(north_east);
    let size = max - min;

    ((min + max) * 0.5, size.xy())
}
fn terrain_skirt_delta(tile_id: CanonicalTileId) -> f32 {
    let bounds = get_tile_lnglat_bounds(tile_id);

    let center = (bounds.0 + bounds.1) * 0.5;
    let frame_delta_meters = EARTH_CIRCUMFERENCE / 2.0_f64.powi(tile_id.z as i32) / 5.0;

    (MercatorCoordinate::from_lng_lat(LngLat::new(center.x, center.y), frame_delta_meters).z
        * MERCATOR_WORLD_SIZE) as f32
}
