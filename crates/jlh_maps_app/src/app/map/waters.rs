use crate::app::map::core::MAP_VIEW_COLOR_RENDER_LAYER;
use crate::app::map::transform::{MERCATOR_WORLD_SIZE, lng_lat_to_world};
use crate::app::maplibre_gl_js::integration::MaplibreMapIntegration;
use crate::app::maplibre_gl_js::types::{
    CanonicalTileId, MaplibreTerrainTileData, SourceLayerFeature,
};
use crate::app::maplibre_gl_js::utils::mercator_coordinate::{LngLat, MercatorCoordinate};
use crate::app::maplibre_gl_js::utils::terrain::get_dem_elevation;
use crate::app::maplibre_gl_js::utils::tile::get_tile_lnglat_bounds;
use crate::utils::debug::SoftExpect;
use bevy::asset::RenderAssetUsages;
use bevy::camera::visibility::RenderLayers;
use bevy::math::{DVec3, dvec2};
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use big_space::grid::Grid;
use geojson::Value;
use serde_json::Value as JsonValue;
use std::collections::{HashMap, HashSet};

pub struct WatersPlugin;

impl Plugin for WatersPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<WaterMaterial>().add_systems(
            Update,
            (sync_spawned_water_buckets, setup_water_bucket_meshes).chain(),
        );
    }
}

const WATER_SOURCE_LAYER: &str = "water";

#[derive(Component)]
pub struct WaterManager {
    pub maplibre_int_id: Entity,
    pub spawned_waters: HashMap<String, SpawnedWaterSource>,
}

#[derive(Default)]
pub struct SpawnedWaterSource {
    tiles: HashMap<CanonicalTileId, Entity>,
}

#[derive(Component)]
struct WaterTileBucket {
    maplibre_int_id: Entity,
    source_id: String,
    tile_id: CanonicalTileId,
    center: DVec3,
    mesh_handle: Option<Handle<Mesh>>,
    mesh_dirty: bool,
    terrain_hash: Option<String>,
    meshed_feature_ids: HashSet<String>,
    buffers: WaterMeshBuffers,
}

#[derive(Default)]
struct WaterMeshBuffers {
    positions: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    uvs: Vec<[f32; 2]>,
    indices: Vec<u32>,
}

impl WaterMeshBuffers {
    fn clear(&mut self) {
        self.positions.clear();
        self.normals.clear();
        self.uvs.clear();
        self.indices.clear();
    }

    fn is_empty(&self) -> bool {
        self.positions.is_empty() || self.indices.is_empty()
    }

    fn to_mesh(&self) -> Mesh {
        let mut mesh = Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        );
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, self.positions.clone());
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, self.normals.clone());
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, self.uvs.clone());
        mesh.insert_indices(Indices::U32(self.indices.clone()));
        mesh
    }
}

#[derive(Resource)]
struct WaterMaterial(Handle<StandardMaterial>);

impl FromWorld for WaterMaterial {
    fn from_world(world: &mut World) -> Self {
        let mut materials = world.resource_mut::<Assets<StandardMaterial>>();
        Self(materials.add(StandardMaterial {
            base_color: Color::srgb(0.2, 0.2, 0.8),
            ..default()
        }))
    }
}

fn sync_spawned_water_buckets(
    mut commands: Commands,
    map_ints: Query<&MaplibreMapIntegration>,
    mut managers: Query<(Entity, &Grid, &mut WaterManager)>,
    mut buckets: Query<&mut WaterTileBucket>,
) {
    for (manager_id, grid, mut manager) in managers.iter_mut() {
        let maplibre_int_id = manager.maplibre_int_id;
        let Some(map_int) = map_ints.get(maplibre_int_id).ok().soft_expect("") else {
            continue;
        };

        remove_stale_water_buckets(
            &mut commands,
            map_int,
            &mut manager.spawned_waters,
            &mut buckets,
        );

        for (source_id, source) in &map_int.features.sources {
            let Some(water_layer) = source.source_layers.get(WATER_SOURCE_LAYER) else {
                continue;
            };

            for tile_id in water_layer.tiles.keys() {
                let spawned_source = manager.spawned_waters.entry(source_id.clone()).or_default();
                if spawned_source.tiles.contains_key(tile_id) {
                    continue;
                }

                let bucket_id = spawn_water_bucket(
                    &mut commands,
                    manager_id,
                    maplibre_int_id,
                    grid,
                    source_id,
                    *tile_id,
                );
                spawned_source.tiles.insert(*tile_id, bucket_id);
            }
        }
    }
}

fn remove_stale_water_buckets(
    commands: &mut Commands,
    map_int: &MaplibreMapIntegration,
    spawned_waters: &mut HashMap<String, SpawnedWaterSource>,
    buckets: &mut Query<&mut WaterTileBucket>,
) {
    spawned_waters.retain(|source_id, spawned_source| {
        let water_layer = map_int
            .features
            .sources
            .get(source_id)
            .and_then(|source| source.source_layers.get(WATER_SOURCE_LAYER));

        spawned_source.tiles.retain(|tile_id, bucket_entity| {
            let Some(current_features) = water_layer.and_then(|layer| layer.tiles.get(tile_id))
            else {
                commands.entity(*bucket_entity).despawn();
                return false;
            };

            let Ok(mut bucket) = buckets.get_mut(*bucket_entity) else {
                return true;
            };

            let removed_meshed_feature = bucket
                .meshed_feature_ids
                .iter()
                .any(|feature_id| !current_features.contains_key(feature_id));

            if removed_meshed_feature {
                bucket.buffers.clear();
                bucket.mesh_dirty = true;
                bucket.meshed_feature_ids.clear();
            }

            true
        });

        !spawned_source.tiles.is_empty()
    });
}

fn spawn_water_bucket(
    commands: &mut Commands,
    manager_id: Entity,
    maplibre_int_id: Entity,
    grid: &Grid,
    source_id: &str,
    tile_id: CanonicalTileId,
) -> Entity {
    let center = tile_flat_center_world(tile_id);
    let (cell, translation) = grid.translation_to_grid(center.with_z(center.z + 0.0001));

    let bucket_id = commands
        .spawn((
            Name::new(format!("Water bucket {source_id}/{tile_id:?}")),
            cell,
            Transform::from_translation(translation),
            RenderLayers::layer(MAP_VIEW_COLOR_RENDER_LAYER),
            WaterTileBucket {
                maplibre_int_id,
                source_id: source_id.to_owned(),
                tile_id,
                center,
                mesh_handle: None,
                mesh_dirty: false,
                terrain_hash: None,
                meshed_feature_ids: HashSet::default(),
                buffers: WaterMeshBuffers::default(),
            },
        ))
        .id();

    commands.entity(manager_id).add_child(bucket_id);
    bucket_id
}

fn setup_water_bucket_meshes(
    mut commands: Commands,
    map_ints: Query<&MaplibreMapIntegration>,
    mut buckets: Query<(Entity, &mut WaterTileBucket)>,
    mut meshes: ResMut<Assets<Mesh>>,
    material: Res<WaterMaterial>,
) {
    for (bucket_entity, mut bucket) in buckets.iter_mut() {
        let Some(map_int) = map_ints.get(bucket.maplibre_int_id).ok() else {
            continue;
        };

        let Some(features) = map_int
            .features
            .sources
            .get(&bucket.source_id)
            .and_then(|source| source.source_layers.get(WATER_SOURCE_LAYER))
            .and_then(|layer| layer.tiles.get(&bucket.tile_id))
        else {
            continue;
        };

        let terrain_data = map_int.terrain.tiles.get(&bucket.tile_id);
        let terrain_hash = terrain_data.map(|terrain_data| terrain_data.hash.clone());
        if terrain_hash != bucket.terrain_hash {
            bucket.buffers.clear();
            bucket.meshed_feature_ids.clear();
            bucket.terrain_hash = terrain_hash;
            bucket.mesh_dirty = true;
        }

        let new_feature_ids = features
            .keys()
            .filter(|feature_id| !bucket.meshed_feature_ids.contains(*feature_id))
            .cloned()
            .collect::<Vec<_>>();

        if new_feature_ids.is_empty() && !bucket.mesh_dirty {
            continue;
        }

        for feature_id in new_feature_ids {
            let Some(feature) = features.get(&feature_id) else {
                continue;
            };
            if append_water_feature_mesh(
                feature,
                bucket.tile_id,
                bucket.center,
                terrain_data,
                &mut bucket.buffers,
            ) {
                bucket.mesh_dirty = true;
                bucket.meshed_feature_ids.insert(feature_id);
            }
        }

        if bucket.buffers.is_empty() {
            if bucket.mesh_dirty {
                commands
                    .entity(bucket_entity)
                    .remove::<(Mesh3d, MeshMaterial3d<StandardMaterial>)>();
                bucket.mesh_handle = None;
                bucket.mesh_dirty = false;
            }
            continue;
        }

        let mesh = bucket.buffers.to_mesh();
        if let Some(mesh_handle) = &bucket.mesh_handle {
            if let Some(existing_mesh) = meshes.get_mut(mesh_handle) {
                *existing_mesh = mesh;
            }
        } else {
            let mesh_handle = meshes.add(mesh);
            bucket.mesh_handle = Some(mesh_handle.clone());
            commands
                .entity(bucket_entity)
                .insert((Mesh3d(mesh_handle), MeshMaterial3d(material.0.clone())));
        }
        bucket.mesh_dirty = false;
    }
}

fn append_water_feature_mesh(
    feature: &SourceLayerFeature,
    tile_id: CanonicalTileId,
    center: DVec3,
    terrain_data: Option<&MaplibreTerrainTileData>,
    buffers: &mut WaterMeshBuffers,
) -> bool {
    let altitude = water_altitude_property(&feature.properties, ["render_height", "height", "ele"])
        .unwrap_or(0.0);
    let start_position_count = buffers.positions.len();
    let start_index_count = buffers.indices.len();

    match &feature.geometry.value {
        Value::Polygon(polygon) => push_polygon_mesh(
            polygon,
            center,
            tile_id,
            altitude,
            terrain_data,
            &mut buffers.positions,
            &mut buffers.normals,
            &mut buffers.uvs,
            &mut buffers.indices,
        ),
        Value::MultiPolygon(polygons) => {
            for polygon in polygons {
                push_polygon_mesh(
                    polygon,
                    center,
                    tile_id,
                    altitude,
                    terrain_data,
                    &mut buffers.positions,
                    &mut buffers.normals,
                    &mut buffers.uvs,
                    &mut buffers.indices,
                );
            }
        }
        _ => return false,
    }

    if buffers.positions.len() == start_position_count || buffers.indices.len() == start_index_count
    {
        buffers.positions.truncate(start_position_count);
        buffers.normals.truncate(start_position_count);
        buffers.uvs.truncate(start_position_count);
        buffers.indices.truncate(start_index_count);
        false
    } else {
        true
    }
}

fn water_altitude_property<const N: usize>(
    properties: &HashMap<String, JsonValue>,
    keys: [&str; N],
) -> Option<f64> {
    keys.into_iter()
        .find_map(|key| properties.get(key).and_then(json_value_as_f64))
}

fn json_value_as_f64(value: &JsonValue) -> Option<f64> {
    match value {
        JsonValue::Number(value) => value.as_f64(),
        JsonValue::String(value) => value.parse().ok(),
        _ => None,
    }
}

struct TerrainElevationTile<'a> {
    bounds: (bevy::math::DVec2, bevy::math::DVec2),
    terrain_data: &'a MaplibreTerrainTileData,
}

impl<'a> TerrainElevationTile<'a> {
    fn new(tile_id: CanonicalTileId, terrain_data: &'a MaplibreTerrainTileData) -> Self {
        Self {
            bounds: get_tile_lnglat_bounds(tile_id),
            terrain_data,
        }
    }

    fn elevation_meters(&self, lnglat: bevy::math::DVec2) -> Option<f64> {
        if !lnglat_bounds_contains(self.bounds, lnglat) {
            return None;
        }

        let bounds_size = self.bounds.1 - self.bounds.0;
        if bounds_size.x == 0.0 || bounds_size.y == 0.0 {
            return None;
        }

        let uv = ((lnglat - self.bounds.0) / bounds_size).as_vec2();
        let uv = vec2(uv.x, 1.0 - uv.y);
        get_dem_elevation(&self.terrain_data.terrain_data, uv).map(f64::from)
    }
}

fn lnglat_bounds_contains(
    bounds: (bevy::math::DVec2, bevy::math::DVec2),
    lnglat: bevy::math::DVec2,
) -> bool {
    lnglat.x >= bounds.0.x
        && lnglat.x <= bounds.1.x
        && lnglat.y >= bounds.0.y
        && lnglat.y <= bounds.1.y
}

fn lng_lat_alt_to_world(lng: f64, lat: f64, alt: f64) -> DVec3 {
    let coords = MercatorCoordinate::from_lng_lat(LngLat::new(lng, lat), alt);
    DVec3::new(coords.x, -coords.y, coords.z) * MERCATOR_WORLD_SIZE
}

fn tile_flat_center_world(tile_id: CanonicalTileId) -> DVec3 {
    let bounds = get_tile_lnglat_bounds(tile_id);
    let south_west = lng_lat_to_world(bounds.0.x, bounds.0.y, 0.0);
    let north_east = lng_lat_to_world(bounds.1.x, bounds.1.y, 0.0);
    let min = south_west.min(north_east);
    let max = south_west.max(north_east);

    (min + max) * 0.5
}

fn push_polygon_mesh(
    polygon: &[Vec<Vec<f64>>],
    center: DVec3,
    tile_id: CanonicalTileId,
    altitude: f64,
    terrain_data: Option<&MaplibreTerrainTileData>,
    positions: &mut Vec<[f32; 3]>,
    normals: &mut Vec<[f32; 3]>,
    uvs: &mut Vec<[f32; 2]>,
    indices: &mut Vec<u32>,
) {
    let first_vertex = positions.len() as u32;
    let mut flat_coords = Vec::new();
    let mut hole_indices = Vec::new();
    let mut vertex_count = 0usize;
    let terrain_tile =
        terrain_data.map(|terrain_data| TerrainElevationTile::new(tile_id, terrain_data));

    for (ring_index, ring) in polygon.iter().enumerate() {
        let ring_positions = ring_without_closing_position(ring);
        let lnglats = ring_positions
            .iter()
            .filter_map(|position| {
                if position.len() >= 2 {
                    Some(dvec2(position[0], position[1]))
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        if lnglats.len() < 3 {
            continue;
        }

        if ring_index > 0 {
            hole_indices.push(vertex_count);
        }

        for lnglat in lnglats {
            let terrain_altitude = terrain_tile
                .as_ref()
                .and_then(|terrain_tile| terrain_tile.elevation_meters(lnglat))
                .unwrap_or(0.0);
            let world =
                lng_lat_alt_to_world(lnglat.x, lnglat.y, altitude + terrain_altitude) - center;
            flat_coords.push(world.x);
            flat_coords.push(world.y);
            positions.push(world.as_vec3().to_array());
            normals.push([0.0, 0.0, 1.0]);
            uvs.push([world.x as f32, world.y as f32]);
            vertex_count += 1;
        }
    }

    if vertex_count < 3 {
        positions.truncate(first_vertex as usize);
        normals.truncate(first_vertex as usize);
        uvs.truncate(first_vertex as usize);
        return;
    }

    let Ok(triangle_indices) = earcutr::earcut(&flat_coords, &hole_indices, 2) else {
        positions.truncate(first_vertex as usize);
        normals.truncate(first_vertex as usize);
        uvs.truncate(first_vertex as usize);
        return;
    };

    indices.extend(
        triangle_indices
            .into_iter()
            .filter_map(|index| u32::try_from(index).ok())
            .map(|index| first_vertex + index),
    );
}

fn ring_without_closing_position(ring: &[Vec<f64>]) -> &[Vec<f64>] {
    let Some((first, rest)) = ring.split_first() else {
        return ring;
    };
    let Some(last) = rest.last() else {
        return ring;
    };

    if positions_equal_2d(first, last) {
        &ring[..ring.len() - 1]
    } else {
        ring
    }
}

fn positions_equal_2d(left: &[f64], right: &[f64]) -> bool {
    left.len() >= 2
        && right.len() >= 2
        && (left[0] - right[0]).abs() <= f64::EPSILON
        && (left[1] - right[1]).abs() <= f64::EPSILON
}
