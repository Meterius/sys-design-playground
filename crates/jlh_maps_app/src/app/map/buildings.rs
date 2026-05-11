use crate::app::map::camera::MapViewCamera;
use crate::app::map::core::MAP_VIEW_COLOR_RENDER_LAYER;
use crate::app::map::transform::lng_lat_to_world;
use crate::app::maplibre_gl_js::integration::MaplibreMapIntegration;
use crate::app::maplibre_gl_js::types::{CanonicalTileId, SourceLayerFeature};
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

pub struct BuildingsPlugin;

impl Plugin for BuildingsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<BuildingMaterial>().add_systems(
            Update,
            (
                sync_spawned_building_buckets,
                sync_distance_visibility,
                setup_visible_building_bucket_meshes,
            )
                .chain(),
        );
    }
}

const DEFAULT_BUILDING_VISIBILITY_DISTANCE: f32 = 10.0;
const BUILDING_SOURCE_LAYER: &str = "building";

#[derive(Component)]
pub struct BuildingManager {
    pub maplibre_int_id: Entity,
    pub spawned_buildings: HashMap<String, SpawnedBuildingSource>,
}

#[derive(Default)]
pub struct SpawnedBuildingSource {
    tiles: HashMap<CanonicalTileId, Entity>,
}

#[derive(Component)]
struct BuildingTileBucket {
    maplibre_int_id: Entity,
    source_id: String,
    tile_id: CanonicalTileId,
    center: DVec3,
    mesh_handle: Option<Handle<Mesh>>,
    mesh_dirty: bool,
    meshed_feature_ids: HashSet<String>,
    buffers: BuildingMeshBuffers,
}

#[derive(Default)]
struct BuildingMeshBuffers {
    positions: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    uvs: Vec<[f32; 2]>,
    indices: Vec<u32>,
}

impl BuildingMeshBuffers {
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
struct BuildingMaterial(Handle<StandardMaterial>);

impl FromWorld for BuildingMaterial {
    fn from_world(world: &mut World) -> Self {
        let mut materials = world.resource_mut::<Assets<StandardMaterial>>();
        Self(materials.add(StandardMaterial {
            base_color: Color::srgb(0.8, 0.2, 0.2),
            ..default()
        }))
    }
}

fn sync_spawned_building_buckets(
    mut commands: Commands,
    map_ints: Query<&MaplibreMapIntegration>,
    mut managers: Query<(Entity, &Grid, &mut BuildingManager)>,
    mut buckets: Query<&mut BuildingTileBucket>,
) {
    for (manager_id, grid, mut manager) in managers.iter_mut() {
        let maplibre_int_id = manager.maplibre_int_id;
        let Some(map_int) = map_ints.get(maplibre_int_id).ok().soft_expect("") else {
            continue;
        };

        remove_stale_building_buckets(
            &mut commands,
            map_int,
            &mut manager.spawned_buildings,
            &mut buckets,
        );

        for (source_id, source) in &map_int.features.sources {
            let Some(building_layer) = source.source_layers.get(BUILDING_SOURCE_LAYER) else {
                continue;
            };

            for tile_id in building_layer.tiles.keys() {
                let spawned_source = manager
                    .spawned_buildings
                    .entry(source_id.clone())
                    .or_default();
                if spawned_source.tiles.contains_key(tile_id) {
                    continue;
                }

                let bucket_id = spawn_building_bucket(
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

fn remove_stale_building_buckets(
    commands: &mut Commands,
    map_int: &MaplibreMapIntegration,
    spawned_buildings: &mut HashMap<String, SpawnedBuildingSource>,
    buckets: &mut Query<&mut BuildingTileBucket>,
) {
    spawned_buildings.retain(|source_id, spawned_source| {
        let building_layer = map_int
            .features
            .sources
            .get(source_id)
            .and_then(|source| source.source_layers.get(BUILDING_SOURCE_LAYER));

        spawned_source.tiles.retain(|tile_id, bucket_entity| {
            let Some(current_features) = building_layer.and_then(|layer| layer.tiles.get(tile_id))
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

fn spawn_building_bucket(
    commands: &mut Commands,
    manager_id: Entity,
    maplibre_int_id: Entity,
    grid: &Grid,
    source_id: &str,
    tile_id: CanonicalTileId,
) -> Entity {
    let (center, flat_half_extents) = tile_flat_bounds_world(tile_id);
    let (cell, translation) = grid.translation_to_grid(center);

    let bucket_id = commands
        .spawn((
            Name::new(format!("Building bucket {source_id}/{tile_id:?}")),
            Visibility::Hidden,
            cell,
            Transform::from_translation(translation),
            RenderLayers::layer(MAP_VIEW_COLOR_RENDER_LAYER),
            DistanceVisibility {
                max_distance: DEFAULT_BUILDING_VISIBILITY_DISTANCE,
                flat_half_extents,
            },
            BuildingTileBucket {
                maplibre_int_id,
                source_id: source_id.to_owned(),
                tile_id,
                center,
                mesh_handle: None,
                mesh_dirty: false,
                meshed_feature_ids: HashSet::default(),
                buffers: BuildingMeshBuffers::default(),
            },
        ))
        .id();

    commands.entity(manager_id).add_child(bucket_id);
    bucket_id
}

pub const DISABLE_DISTANCE_VISIBILITY: bool = false;

#[derive(Component)]
pub struct DistanceVisibility {
    pub max_distance: f32,
    pub flat_half_extents: Vec2,
}

fn sync_distance_visibility(
    cameras: Query<(&MapViewCamera, &GlobalTransform, &ChildOf)>,
    mut distance_visible_entities: Query<
        (
            &DistanceVisibility,
            &GlobalTransform,
            &ChildOf,
            &mut Visibility,
        ),
        Without<MapViewCamera>,
    >,
) {
    for (distance_visibility, entity_transform, ChildOf(entity_parent), mut visibility) in
        distance_visible_entities.iter_mut()
    {
        let Some((_, camera_transform, _)) = cameras
            .iter()
            .find(|(_, _, ChildOf(camera_parent))| camera_parent == entity_parent)
        else {
            continue;
        };

        let max_distance_squared =
            distance_visibility.max_distance * distance_visibility.max_distance;
        let distance_squared = distance_to_flat_bounds_squared(
            camera_transform.translation(),
            entity_transform.translation(),
            distance_visibility.flat_half_extents,
        );

        *visibility = if DISABLE_DISTANCE_VISIBILITY || distance_squared <= max_distance_squared {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
}

fn setup_visible_building_bucket_meshes(
    mut commands: Commands,
    map_ints: Query<&MaplibreMapIntegration>,
    mut buckets: Query<(Entity, &mut BuildingTileBucket, &Visibility)>,
    mut meshes: ResMut<Assets<Mesh>>,
    material: Res<BuildingMaterial>,
) {
    for (bucket_entity, mut bucket, visibility) in buckets.iter_mut() {
        if matches!(*visibility, Visibility::Hidden) {
            continue;
        }

        let Some(features) = map_ints
            .get(bucket.maplibre_int_id)
            .ok()
            .and_then(|map_int| map_int.features.sources.get(&bucket.source_id))
            .and_then(|source| source.source_layers.get(BUILDING_SOURCE_LAYER))
            .and_then(|layer| layer.tiles.get(&bucket.tile_id))
        else {
            continue;
        };

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
            if append_building_feature_mesh(feature, bucket.center, &mut bucket.buffers) {
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

fn append_building_feature_mesh(
    feature: &SourceLayerFeature,
    center: DVec3,
    buffers: &mut BuildingMeshBuffers,
) -> bool {
    let base_altitude =
        building_altitude_property(&feature.properties, ["render_min_height", "min_height"])
            .unwrap_or(0.0);
    let top_altitude =
        building_altitude_property(&feature.properties, ["render_height", "height"]).unwrap_or(0.0);
    let top_altitude = top_altitude.max(base_altitude);
    let start_position_count = buffers.positions.len();
    let start_index_count = buffers.indices.len();

    match &feature.geometry.value {
        Value::Polygon(polygon) => push_polygon_mesh(
            polygon,
            center,
            base_altitude,
            top_altitude,
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
                    base_altitude,
                    top_altitude,
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

fn building_altitude_property<const N: usize>(
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

fn tile_flat_bounds_world(tile_id: CanonicalTileId) -> (DVec3, Vec2) {
    let bounds = get_tile_lnglat_bounds(tile_id);
    let south_west = lng_lat_to_world(bounds.0.x, bounds.0.y, 0.0);
    let north_east = lng_lat_to_world(bounds.1.x, bounds.1.y, 0.0);
    let min = south_west.min(north_east);
    let max = south_west.max(north_east);
    let size = max - min;

    ((min + max) * 0.5, size.xy().as_vec2() * 0.5)
}

fn distance_to_flat_bounds_squared(point: Vec3, center: Vec3, half_extents: Vec2) -> f32 {
    let local = point - center;
    let outside_xy = (local.xy().abs() - half_extents).max(Vec2::ZERO);

    outside_xy.length_squared() + local.z * local.z
}

fn push_polygon_mesh(
    polygon: &[Vec<Vec<f64>>],
    center: DVec3,
    start_altitude: f64,
    end_altitude: f64,
    positions: &mut Vec<[f32; 3]>,
    normals: &mut Vec<[f32; 3]>,
    uvs: &mut Vec<[f32; 2]>,
    indices: &mut Vec<u32>,
) {
    let first_vertex = positions.len() as u32;
    let mut flat_coords = Vec::new();
    let mut hole_indices = Vec::new();
    let mut vertex_count = 0usize;
    let mut rings = Vec::new();

    for (ring_index, ring) in polygon.iter().enumerate() {
        let ring_positions = ring_without_closing_position(ring);
        if ring_positions.len() < 3 {
            continue;
        }

        if ring_index > 0 {
            hole_indices.push(vertex_count);
        }

        let ring_first_vertex = positions.len();
        let mut ring_world_positions = Vec::new();
        for position in ring_positions {
            if position.len() < 2 {
                continue;
            }

            let lnglat = dvec2(position[0], position[1]);
            let top_world = lng_lat_to_world(lnglat.x, lnglat.y, end_altitude) - center;
            let base_world = lng_lat_to_world(lnglat.x, lnglat.y, start_altitude) - center;
            flat_coords.push(top_world.x);
            flat_coords.push(top_world.y);
            positions.push(top_world.as_vec3().to_array());
            normals.push([0.0, 0.0, 1.0]);
            uvs.push([top_world.x as f32, top_world.y as f32]);
            ring_world_positions.push(ExtrusionVertex {
                top: top_world,
                base: base_world,
            });
            vertex_count += 1;
        }

        let ring_vertex_count = positions.len() - ring_first_vertex;
        if ring_vertex_count >= 3 {
            rings.push(ring_world_positions);
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

    for ring_world_positions in rings {
        push_extrusion_wall_mesh(&ring_world_positions, positions, normals, uvs, indices);
    }
}

struct ExtrusionVertex {
    top: DVec3,
    base: DVec3,
}

fn push_extrusion_wall_mesh(
    ring_positions: &[ExtrusionVertex],
    positions: &mut Vec<[f32; 3]>,
    normals: &mut Vec<[f32; 3]>,
    uvs: &mut Vec<[f32; 2]>,
    indices: &mut Vec<u32>,
) {
    let ring_len = ring_positions.len();
    if ring_len < 2 {
        return;
    }

    let ring_area = signed_area_xy(ring_positions);
    for edge_index in 0..ring_len {
        let next_edge_index = (edge_index + 1) % ring_len;
        let top_left = ring_positions[edge_index].top;
        let top_right = ring_positions[next_edge_index].top;
        let edge = top_right - top_left;
        let normal = if ring_area >= 0.0 {
            DVec3::new(edge.y, -edge.x, 0.0)
        } else {
            DVec3::new(-edge.y, edge.x, 0.0)
        }
        .normalize_or_zero()
        .as_vec3()
        .to_array();

        let base_left = ring_positions[edge_index].base;
        let base_right = ring_positions[next_edge_index].base;

        let first_wall_vertex = positions.len() as u32;
        positions.push(top_left.as_vec3().to_array());
        positions.push(top_right.as_vec3().to_array());
        positions.push(base_right.as_vec3().to_array());
        positions.push(base_left.as_vec3().to_array());
        normals.extend([normal, normal, normal, normal]);
        uvs.push([edge_index as f32, 1.0]);
        uvs.push([next_edge_index as f32, 1.0]);
        uvs.push([next_edge_index as f32, 0.0]);
        uvs.push([edge_index as f32, 0.0]);

        if ring_area >= 0.0 {
            indices.extend([
                first_wall_vertex,
                first_wall_vertex + 2,
                first_wall_vertex + 1,
                first_wall_vertex,
                first_wall_vertex + 3,
                first_wall_vertex + 2,
            ]);
        } else {
            indices.extend([
                first_wall_vertex,
                first_wall_vertex + 1,
                first_wall_vertex + 2,
                first_wall_vertex,
                first_wall_vertex + 2,
                first_wall_vertex + 3,
            ]);
        }
    }
}

fn signed_area_xy(positions: &[ExtrusionVertex]) -> f64 {
    positions
        .iter()
        .zip(positions.iter().cycle().skip(1))
        .take(positions.len())
        .map(|(left, right)| left.top.x * right.top.y - right.top.x * left.top.y)
        .sum::<f64>()
        * 0.5
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
