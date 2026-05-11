use crate::app::common::materials::DepthTextureMaterial;
use crate::app::map::camera::MapViewCamera;
use crate::app::map::core::{MAP_VIEW_COLOR_RENDER_LAYER, MAP_VIEW_DEPTH_RENDER_LAYER};
use crate::app::map::transform::lng_lat_to_world;
use crate::app::maplibre_gl_js::integration::MaplibreMapIntegration;
use crate::app::maplibre_gl_js::types::{CanonicalTileId, SourceLayerFeature};
use crate::utils::debug::SoftExpect;
use bevy::asset::RenderAssetUsages;
use bevy::camera::visibility::RenderLayers;
use bevy::math::{DVec2, DVec3, dvec2};
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use big_space::grid::Grid;
use geojson::{Geometry, Value};
use serde_json::Value as JsonValue;
use std::collections::HashMap;

pub struct BuildingsPlugin;

impl Plugin for BuildingsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                sync_spawned_buildings,
                setup_buildings,
                sync_distance_visibility,
            )
                .chain(),
        );
    }
}

const DEFAULT_BUILDING_VISIBILITY_DISTANCE: f32 = 30.0;

#[derive(Component)]
pub struct BuildingManager {
    pub maplibre_int_id: Entity,
    pub spawned_buildings: HashMap<String, SpawnedBuildingSource>,
}

#[derive(Default)]
pub struct SpawnedBuildingSource {
    tiles: HashMap<CanonicalTileId, HashMap<String, Entity>>,
}

fn sync_spawned_buildings(
    mut commands: Commands,
    map_ints: Query<&MaplibreMapIntegration>,
    mut managers: Query<(Entity, &mut BuildingManager)>,
) {
    for (manager_id, mut manager) in managers.iter_mut() {
        let Some(map_int) = map_ints.get(manager.maplibre_int_id).ok().soft_expect("") else {
            continue;
        };

        remove_stale_buildings(&mut commands, map_int, &mut manager.spawned_buildings);

        for (source_id, source) in &map_int.features.sources {
            let Some(building_layer) = source.source_layers.get("building") else {
                continue;
            };

            for (tile_id, features) in &building_layer.tiles {
                for (feature_id, feature) in features {
                    let spawned_source = manager
                        .spawned_buildings
                        .entry(source_id.clone())
                        .or_default();
                    let spawned_tile = spawned_source.tiles.entry(*tile_id).or_default();
                    if spawned_tile.contains_key(feature_id) {
                        continue;
                    }

                    let building_id = spawn_building(
                        &mut commands,
                        manager_id,
                        source_id,
                        *tile_id,
                        feature_id,
                        feature,
                    );
                    spawned_tile.insert(feature_id.clone(), building_id);
                }
            }
        }
    }
}

fn remove_stale_buildings(
    commands: &mut Commands,
    map_int: &MaplibreMapIntegration,
    spawned_buildings: &mut HashMap<String, SpawnedBuildingSource>,
) {
    spawned_buildings.retain(|source_id, spawned_source| {
        let building_layer = map_int
            .features
            .sources
            .get(source_id)
            .and_then(|source| source.source_layers.get("building"));

        spawned_source.tiles.retain(|tile_id, spawned_tile| {
            let current_features = building_layer.and_then(|layer| layer.tiles.get(tile_id));

            spawned_tile.retain(|feature_id, building_entity| {
                if current_features.is_some_and(|features| features.contains_key(feature_id)) {
                    true
                } else {
                    commands.entity(*building_entity).despawn();
                    false
                }
            });

            !spawned_tile.is_empty()
        });

        !spawned_source.tiles.is_empty()
    });
}

fn spawn_building(
    commands: &mut Commands,
    manager_id: Entity,
    source_id: &str,
    tile_id: CanonicalTileId,
    feature_id: &str,
    feature: &SourceLayerFeature,
) -> Entity {
    let mut building_commands = commands.spawn((
        Name::new(format!("Building {source_id}/{tile_id:?}/{feature_id}")),
        Visibility::Inherited,
        Building {
            geometry: feature.geometry.clone(),
            base_altitude: building_altitude_property(
                &feature.properties,
                ["render_min_height", "min_height"],
            )
            .unwrap_or(0.0),
            top_altitude: building_altitude_property(
                &feature.properties,
                ["render_height", "height"],
            )
            .unwrap_or(0.0),
        },
    ));

    building_commands.insert(DistanceVisibility {
        max_distance: DEFAULT_BUILDING_VISIBILITY_DISTANCE,
    });

    let building_id = building_commands.id();

    commands.entity(manager_id).add_child(building_id);
    building_id
}

#[derive(Component)]
pub struct Building {
    geometry: Geometry,
    base_altitude: f64,
    top_altitude: f64,
}

pub const DISABLE_DISTANCE_VISIBILITY: bool = false;

#[derive(Component)]
pub struct DistanceVisibility {
    pub max_distance: f32,
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
        let distance_squared = entity_transform
            .translation()
            .distance_squared(camera_transform.translation());

        *visibility = if DISABLE_DISTANCE_VISIBILITY || distance_squared <= max_distance_squared {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
}

fn setup_buildings(
    mut commands: Commands,
    buildings: Query<(Entity, &Building, &ChildOf), Added<Building>>,
    grids: Query<&Grid>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut standard_materials: ResMut<Assets<StandardMaterial>>,
    mut depth_materials: ResMut<Assets<DepthTextureMaterial>>,
) {
    for (building_id, building, ChildOf(building_parent_id)) in buildings.iter() {
        let Some(grid) = grids.get(*building_parent_id).ok().soft_expect("") else {
            continue;
        };

        let Some(center_lnglat) = geometry_center_lnglat(&building.geometry) else {
            continue;
        };

        let center = lng_lat_to_world(center_lnglat.x, center_lnglat.y, 0.0);
        let (feature_cell, feature_translation) = grid.translation_to_grid(center);

        let mesh = build_feature_polygon_mesh(
            &building.geometry,
            center,
            building.base_altitude,
            building.top_altitude,
        )
        .map(|mesh| meshes.add(mesh))
        .unwrap_or_default();

        commands.entity(building_id).insert((
            feature_cell,
            Transform::from_translation(feature_translation),
            Mesh3d(mesh.clone()),
            MeshMaterial3d(standard_materials.add(StandardMaterial {
                base_color: Color::srgb(0.8, 0.2, 0.2),
                ..default()
            })),
            RenderLayers::layer(MAP_VIEW_COLOR_RENDER_LAYER),
            DistanceVisibility {
                max_distance: DEFAULT_BUILDING_VISIBILITY_DISTANCE,
            },
        ));

        commands.entity(building_id).with_children(|children| {
            children.spawn((
                Name::new("Building Depth Mesh"),
                Visibility::Inherited,
                Transform::default(),
                Mesh3d(mesh),
                MeshMaterial3d(depth_materials.add(DepthTextureMaterial {})),
                RenderLayers::layer(MAP_VIEW_DEPTH_RENDER_LAYER),
            ));
        });
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

fn geometry_center_lnglat(geometry: &Geometry) -> Option<DVec2> {
    let mut min = DVec2::splat(f64::INFINITY);
    let mut max = DVec2::splat(f64::NEG_INFINITY);
    let mut has_position = false;

    visit_geometry_positions(geometry, &mut |position| {
        if position.len() < 2 {
            return;
        }

        let lnglat = dvec2(position[0], position[1]);
        min = min.min(lnglat);
        max = max.max(lnglat);
        has_position = true;
    });

    has_position.then_some((min + max) * 0.5)
}

fn visit_geometry_positions(geometry: &Geometry, visit: &mut impl FnMut(&Vec<f64>)) {
    match &geometry.value {
        Value::Point(position) => visit(position),
        Value::MultiPoint(positions) | Value::LineString(positions) => {
            for position in positions {
                visit(position);
            }
        }
        Value::MultiLineString(lines) | Value::Polygon(lines) => {
            for line in lines {
                for position in line {
                    visit(position);
                }
            }
        }
        Value::MultiPolygon(polygons) => {
            for polygon in polygons {
                for line in polygon {
                    for position in line {
                        visit(position);
                    }
                }
            }
        }
        Value::GeometryCollection(geometries) => {
            for geometry in geometries {
                visit_geometry_positions(geometry, visit);
            }
        }
    }
}

fn build_feature_polygon_mesh(
    geometry: &Geometry,
    center: DVec3,
    start_altitude: f64,
    end_altitude: f64,
) -> Option<Mesh> {
    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut uvs = Vec::new();
    let mut indices = Vec::new();
    let end_altitude = end_altitude.max(start_altitude);

    match &geometry.value {
        Value::Polygon(polygon) => push_polygon_mesh(
            polygon,
            center,
            start_altitude,
            end_altitude,
            &mut positions,
            &mut normals,
            &mut uvs,
            &mut indices,
        ),
        Value::MultiPolygon(polygons) => {
            for polygon in polygons {
                push_polygon_mesh(
                    polygon,
                    center,
                    start_altitude,
                    end_altitude,
                    &mut positions,
                    &mut normals,
                    &mut uvs,
                    &mut indices,
                );
            }
        }
        _ => return None,
    }

    if positions.is_empty() || indices.is_empty() {
        return None;
    }

    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(Indices::U32(indices));
    Some(mesh)
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

            let lnglat = DVec2::new(position[0], position[1]);
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
