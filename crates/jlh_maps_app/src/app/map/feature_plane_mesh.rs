use crate::app::map::transform::{MERCATOR_WORLD_SIZE, lng_lat_to_world};
use crate::app::maplibre_gl_js::integration::MaplibreMapIntegration;
use crate::app::maplibre_gl_js::types::{
    CanonicalTileId, MaplibreTerrainTileData, SourceLayerFeature,
};
use crate::app::maplibre_gl_js::utils::mercator_coordinate::{LngLat, MercatorCoordinate};
use crate::app::maplibre_gl_js::utils::terrain::get_dem_elevation;
use crate::app::maplibre_gl_js::utils::tile::get_tile_lnglat_bounds;
use crate::utils::edge_distance::update_edge_distance_texture;
use bevy::asset::RenderAssetUsages;
use bevy::image::ImageSampler;
use bevy::math::{DVec3, dvec2};
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use geojson::Value;
use serde_json::Value as JsonValue;
use std::collections::{HashMap, HashSet};

const EDGE_DISTANCE_MAX_UV: f32 = 0.01;

pub struct FeaturePlaneMeshPlugin;

impl Plugin for FeaturePlaneMeshPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                handle_removed_feature_tile_bucket_data,
                setup_feature_tile_bucket_edge_distance_textures,
                setup_feature_tile_bucket_plane_meshes,
            )
                .chain(),
        );
    }
}

#[derive(Component)]
pub struct FeatureTileBucket {
    pub maplibre_int_id: Entity,
    pub source_id: String,
    pub source_layer_id: String,
    pub tile_id: CanonicalTileId,
    pub center: DVec3,
}

impl FeatureTileBucket {
    pub fn new(
        maplibre_int_id: Entity,
        source_id: &str,
        source_layer_id: &str,
        tile_id: CanonicalTileId,
        center: DVec3,
    ) -> Self {
        Self {
            maplibre_int_id,
            source_id: source_id.to_owned(),
            source_layer_id: source_layer_id.to_owned(),
            tile_id,
            center,
        }
    }
}

// Mesh

#[derive(Component, Clone, Copy, Default)]
pub struct FeatureTileBucketPlaneMeshConfig {
    pub base_property_keys: Option<&'static [&'static str]>,
    pub top_property_keys: Option<&'static [&'static str]>,
    pub height_color_gradient_strength: f32,
    pub height_color_gradient_upper_altitude: f64,
}

#[derive(Component)]
pub struct FeatureTileBucketPlaneMesh {
    mesh_handle: Option<Handle<Mesh>>,
    mesh_dirty: bool,
    terrain_hash: Option<String>,
    meshed_feature_ids: HashSet<String>,
    buffers: FeaturePlaneMeshBuffers,
}

impl Default for FeatureTileBucketPlaneMesh {
    fn default() -> Self {
        Self {
            mesh_handle: None,
            mesh_dirty: true,
            terrain_hash: None,
            meshed_feature_ids: HashSet::default(),
            buffers: FeaturePlaneMeshBuffers::default(),
        }
    }
}

impl FeatureTileBucketPlaneMesh {
    fn handle_removed_features(&mut self, current_features: &HashMap<String, SourceLayerFeature>) {
        let removed_meshed_feature = self
            .meshed_feature_ids
            .iter()
            .any(|feature_id| !current_features.contains_key(feature_id));

        if removed_meshed_feature {
            self.clear_mesh_data();
        }
    }

    fn clear_mesh_data(&mut self) {
        self.buffers.clear();
        self.mesh_dirty = true;
        self.meshed_feature_ids.clear();
    }
}

// Edge distance texture

#[derive(Component)]
pub struct FeatureTileBucketEdgeDistanceTexture {
    pub texture: Handle<Image>,
    resolution: UVec2,
    data: Vec<f32>,
    dirty: bool,
    rasterized_feature_ids: HashSet<String>,
}

impl FeatureTileBucketEdgeDistanceTexture {
    pub fn new(resolution: UVec2, images: &mut Assets<Image>) -> Self {
        let resolution = resolution.max(UVec2::ONE);
        let data = vec![0.0; (resolution.x * resolution.y) as usize];
        let texture = images.add(edge_distance_image(resolution, &data));

        Self {
            texture,
            resolution,
            data,
            dirty: true,
            rasterized_feature_ids: HashSet::default(),
        }
    }

    fn handle_removed_features(&mut self, current_features: &HashMap<String, SourceLayerFeature>) {
        let removed_rasterized_feature = self
            .rasterized_feature_ids
            .iter()
            .any(|feature_id| !current_features.contains_key(feature_id));

        if removed_rasterized_feature {
            self.clear_raster_data();
        }
    }

    fn clear_raster_data(&mut self) {
        self.data.fill(0.0);
        self.dirty = true;
        self.rasterized_feature_ids.clear();
    }
}

#[derive(Default)]
struct FeaturePlaneMeshBuffers {
    positions: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    uvs: Vec<[f32; 2]>,
    colors: Vec<[f32; 4]>,
    indices: Vec<u32>,
}

impl FeaturePlaneMeshBuffers {
    fn clear(&mut self) {
        self.positions.clear();
        self.normals.clear();
        self.uvs.clear();
        self.colors.clear();
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
        mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, self.colors.clone());
        mesh.insert_indices(Indices::U32(self.indices.clone()));
        mesh
    }
}

fn setup_feature_tile_bucket_plane_mesh(
    commands: &mut Commands,
    map_int: &MaplibreMapIntegration,
    bucket_entity: Entity,
    bucket: &FeatureTileBucket,
    plane_mesh: &mut FeatureTileBucketPlaneMesh,
    meshes: &mut Assets<Mesh>,
    config: FeatureTileBucketPlaneMeshConfig,
) {
    let Some(features) = feature_tile_features(map_int, bucket) else {
        return;
    };

    let terrain_data = map_int.terrain.tiles.get(&bucket.tile_id);
    let terrain_hash = terrain_data.map(|terrain_data| terrain_data.hash.clone());
    if terrain_hash != plane_mesh.terrain_hash {
        plane_mesh.clear_mesh_data();
        plane_mesh.terrain_hash = terrain_hash;
    }

    let new_feature_ids = features
        .keys()
        .filter(|feature_id| !plane_mesh.meshed_feature_ids.contains(*feature_id))
        .cloned()
        .collect::<Vec<_>>();

    if new_feature_ids.is_empty() && !plane_mesh.mesh_dirty {
        return;
    }

    let bounds = get_tile_lnglat_bounds(bucket.tile_id);
    for feature_id in new_feature_ids {
        let Some(feature) = features.get(&feature_id) else {
            continue;
        };
        if append_feature_plane_mesh(
            feature,
            bucket.tile_id,
            bucket.center,
            bounds,
            terrain_data,
            &config,
            &mut plane_mesh.buffers,
        ) {
            plane_mesh.mesh_dirty = true;
            plane_mesh.meshed_feature_ids.insert(feature_id);
        }
    }

    if plane_mesh.buffers.is_empty() {
        if plane_mesh.mesh_dirty {
            commands.entity(bucket_entity).remove::<Mesh3d>();
            plane_mesh.mesh_handle = None;
            plane_mesh.mesh_dirty = false;
        }
        return;
    }

    let mesh = plane_mesh.buffers.to_mesh();
    if let Some(mesh_handle) = &plane_mesh.mesh_handle {
        if let Some(existing_mesh) = meshes.get_mut(mesh_handle) {
            *existing_mesh = mesh;
        }
    } else {
        let mesh_handle = meshes.add(mesh);
        plane_mesh.mesh_handle = Some(mesh_handle.clone());
        commands.entity(bucket_entity).insert(Mesh3d(mesh_handle));
    }
    plane_mesh.mesh_dirty = false;
}

fn setup_feature_tile_bucket_edge_distance_texture(
    map_int: &MaplibreMapIntegration,
    bucket: &FeatureTileBucket,
    edge_texture: &mut FeatureTileBucketEdgeDistanceTexture,
    images: &mut Assets<Image>,
) {
    let Some(features) = feature_tile_features(map_int, bucket) else {
        return;
    };

    let new_feature_ids = features
        .keys()
        .filter(|feature_id| !edge_texture.rasterized_feature_ids.contains(*feature_id))
        .cloned()
        .collect::<Vec<_>>();

    if new_feature_ids.is_empty() && !edge_texture.dirty {
        return;
    }

    let bounds = get_tile_lnglat_bounds(bucket.tile_id);
    calculate_edge_distance_texture(bounds, features, edge_texture);
    if let Some(image) = images.get_mut(&edge_texture.texture) {
        *image = edge_distance_image(edge_texture.resolution, &edge_texture.data);
    }
    edge_texture.dirty = false;
}

fn handle_removed_features(
    map_int: Option<&MaplibreMapIntegration>,
    bucket: &FeatureTileBucket,
    plane_mesh: Option<&mut FeatureTileBucketPlaneMesh>,
    edge_texture: Option<&mut FeatureTileBucketEdgeDistanceTexture>,
) -> bool {
    let Some(map_int) = map_int else {
        return false;
    };
    let Some(current_features) = feature_tile_features(map_int, bucket) else {
        return false;
    };

    if let Some(plane_mesh) = plane_mesh {
        plane_mesh.handle_removed_features(current_features);
    }
    if let Some(edge_texture) = edge_texture {
        edge_texture.handle_removed_features(current_features);
    }
    true
}

fn handle_removed_feature_tile_bucket_data(
    map_ints: Query<&MaplibreMapIntegration>,
    mut buckets: Query<(
        &FeatureTileBucket,
        Option<&mut FeatureTileBucketPlaneMesh>,
        Option<&mut FeatureTileBucketEdgeDistanceTexture>,
    )>,
) {
    for (bucket, mut plane_mesh, mut edge_texture) in buckets.iter_mut() {
        handle_removed_features(
            map_ints.get(bucket.maplibre_int_id).ok(),
            bucket,
            plane_mesh.as_deref_mut(),
            edge_texture.as_deref_mut(),
        );
    }
}

fn setup_feature_tile_bucket_plane_meshes(
    mut commands: Commands,
    map_ints: Query<&MaplibreMapIntegration>,
    mut buckets: Query<(
        Entity,
        &FeatureTileBucket,
        &FeatureTileBucketPlaneMeshConfig,
        &mut FeatureTileBucketPlaneMesh,
        Option<&Visibility>,
    )>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    for (bucket_entity, bucket, config, mut plane_mesh, visibility) in buckets.iter_mut() {
        if matches!(visibility, Some(Visibility::Hidden)) {
            continue;
        }

        let Some(map_int) = map_ints.get(bucket.maplibre_int_id).ok() else {
            continue;
        };

        setup_feature_tile_bucket_plane_mesh(
            &mut commands,
            map_int,
            bucket_entity,
            bucket,
            &mut plane_mesh,
            &mut meshes,
            *config,
        );
    }
}

fn setup_feature_tile_bucket_edge_distance_textures(
    map_ints: Query<&MaplibreMapIntegration>,
    mut buckets: Query<(
        &FeatureTileBucket,
        &mut FeatureTileBucketEdgeDistanceTexture,
    )>,
    mut images: ResMut<Assets<Image>>,
) {
    for (bucket, mut edge_texture) in buckets.iter_mut() {
        let Some(map_int) = map_ints.get(bucket.maplibre_int_id).ok() else {
            continue;
        };

        setup_feature_tile_bucket_edge_distance_texture(
            map_int,
            bucket,
            &mut edge_texture,
            &mut images,
        );
    }
}

fn feature_tile_features<'a>(
    map_int: &'a MaplibreMapIntegration,
    bucket: &FeatureTileBucket,
) -> Option<&'a HashMap<String, SourceLayerFeature>> {
    map_int
        .features
        .sources
        .get(&bucket.source_id)
        .and_then(|source| source.source_layers.get(&bucket.source_layer_id))
        .and_then(|layer| layer.tiles.get(&bucket.tile_id))
}

fn append_feature_plane_mesh(
    feature: &SourceLayerFeature,
    tile_id: CanonicalTileId,
    center: DVec3,
    bounds: (bevy::math::DVec2, bevy::math::DVec2),
    terrain_data: Option<&MaplibreTerrainTileData>,
    altitude_config: &FeatureTileBucketPlaneMeshConfig,
    buffers: &mut FeaturePlaneMeshBuffers,
) -> bool {
    let base_altitude = altitude_config
        .base_property_keys
        .and_then(|keys| feature_altitude_property(&feature.properties, keys))
        .unwrap_or(0.0);

    let top_altitude = altitude_config
        .top_property_keys
        .and_then(|keys| feature_altitude_property(&feature.properties, keys))
        .filter(|top_altitude| *top_altitude > base_altitude);

    let start_position_count = buffers.positions.len();
    let start_index_count = buffers.indices.len();

    match &feature.geometry.value {
        Value::Polygon(polygon) => push_polygon_mesh(
            polygon,
            center,
            tile_id,
            bounds,
            base_altitude,
            top_altitude,
            altitude_config.height_color_gradient_strength,
            altitude_config.height_color_gradient_upper_altitude,
            terrain_data,
            &mut buffers.positions,
            &mut buffers.normals,
            &mut buffers.uvs,
            &mut buffers.colors,
            &mut buffers.indices,
        ),
        Value::MultiPolygon(polygons) => {
            for polygon in polygons {
                push_polygon_mesh(
                    polygon,
                    center,
                    tile_id,
                    bounds,
                    base_altitude,
                    top_altitude,
                    altitude_config.height_color_gradient_strength,
                    altitude_config.height_color_gradient_upper_altitude,
                    terrain_data,
                    &mut buffers.positions,
                    &mut buffers.normals,
                    &mut buffers.uvs,
                    &mut buffers.colors,
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
        buffers.colors.truncate(start_position_count);
        buffers.indices.truncate(start_index_count);
        false
    } else {
        true
    }
}

fn feature_altitude_property(
    properties: &HashMap<String, JsonValue>,
    keys: &[&str],
) -> Option<f64> {
    keys.iter()
        .find_map(|key| properties.get(*key).and_then(json_value_as_f64))
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

fn tile_uv(bounds: (bevy::math::DVec2, bevy::math::DVec2), lnglat: bevy::math::DVec2) -> Vec2 {
    let bounds_size = bounds.1 - bounds.0;
    if bounds_size.x == 0.0 || bounds_size.y == 0.0 {
        return Vec2::ZERO;
    }

    let uv = ((lnglat - bounds.0) / bounds_size).as_vec2();
    vec2(uv.x, 1.0 - uv.y)
}

pub fn tile_flat_center_world(tile_id: CanonicalTileId) -> DVec3 {
    tile_flat_bounds_world(tile_id).0
}

pub fn tile_flat_bounds_world(tile_id: CanonicalTileId) -> (DVec3, Vec2) {
    let bounds = get_tile_lnglat_bounds(tile_id);
    let south_west = lng_lat_to_world(bounds.0.x, bounds.0.y, 0.0);
    let north_east = lng_lat_to_world(bounds.1.x, bounds.1.y, 0.0);
    let min = south_west.min(north_east);
    let max = south_west.max(north_east);
    let size = max - min;

    ((min + max) * 0.5, size.xy().as_vec2() * 0.5)
}

fn push_polygon_mesh(
    polygon: &[Vec<Vec<f64>>],
    center: DVec3,
    tile_id: CanonicalTileId,
    bounds: (bevy::math::DVec2, bevy::math::DVec2),
    base_altitude: f64,
    top_altitude: Option<f64>,
    height_color_gradient_strength: f32,
    height_color_gradient_upper_altitude: f64,
    terrain_data: Option<&MaplibreTerrainTileData>,
    positions: &mut Vec<[f32; 3]>,
    normals: &mut Vec<[f32; 3]>,
    uvs: &mut Vec<[f32; 2]>,
    colors: &mut Vec<[f32; 4]>,
    indices: &mut Vec<u32>,
) {
    let first_vertex = positions.len() as u32;
    let mut flat_coords = Vec::new();
    let mut hole_indices = Vec::new();
    let mut vertex_count = 0usize;
    let mut rings = Vec::new();
    let terrain_tile =
        terrain_data.map(|terrain_data| TerrainElevationTile::new(tile_id, terrain_data));
    let surface_altitude = top_altitude.unwrap_or(base_altitude);

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

        let ring_first_vertex = positions.len();
        let mut ring_world_positions = Vec::new();
        for lnglat in lnglats {
            let terrain_altitude = terrain_tile
                .as_ref()
                .and_then(|terrain_tile| terrain_tile.elevation_meters(lnglat))
                .unwrap_or(0.0);
            let world =
                lng_lat_alt_to_world(lnglat.x, lnglat.y, surface_altitude + terrain_altitude)
                    - center;
            flat_coords.push(world.x);
            flat_coords.push(world.y);
            positions.push(world.as_vec3().to_array());
            normals.push([0.0, 0.0, 1.0]);
            uvs.push(tile_uv(bounds, lnglat).to_array());
            colors.push(height_gradient_color(
                surface_altitude + terrain_altitude,
                height_color_gradient_strength,
                height_color_gradient_upper_altitude,
            ));
            if top_altitude.is_some() {
                ring_world_positions.push(ExtrusionVertex {
                    top: world,
                    top_altitude: surface_altitude + terrain_altitude,
                    base: lng_lat_alt_to_world(
                        lnglat.x,
                        lnglat.y,
                        base_altitude + terrain_altitude,
                    ) - center,
                    base_altitude: base_altitude + terrain_altitude,
                });
            }
            vertex_count += 1;
        }

        if top_altitude.is_some() && positions.len() - ring_first_vertex >= 3 {
            rings.push((ring_world_positions, ring_index > 0));
        }
    }

    if vertex_count < 3 {
        positions.truncate(first_vertex as usize);
        normals.truncate(first_vertex as usize);
        uvs.truncate(first_vertex as usize);
        colors.truncate(first_vertex as usize);
        return;
    }

    let Ok(triangle_indices) = earcutr::earcut(&flat_coords, &hole_indices, 2) else {
        positions.truncate(first_vertex as usize);
        normals.truncate(first_vertex as usize);
        uvs.truncate(first_vertex as usize);
        colors.truncate(first_vertex as usize);
        return;
    };

    indices.extend(
        triangle_indices
            .into_iter()
            .filter_map(|index| u32::try_from(index).ok())
            .map(|index| first_vertex + index),
    );

    for (ring_world_positions, is_hole) in rings {
        push_extrusion_wall_mesh(
            &ring_world_positions,
            is_hole,
            positions,
            normals,
            uvs,
            colors,
            indices,
            height_color_gradient_strength,
            height_color_gradient_upper_altitude,
        );
    }
}

fn height_gradient_color(altitude: f64, strength: f32, upper_altitude: f64) -> [f32; 4] {
    let height_fraction = (altitude / upper_altitude.max(f64::EPSILON)).clamp(0.0, 1.0) as f32;
    let value = 1.0 + strength.max(0.0) * height_fraction;
    [value, value, value, 1.0]
}

struct ExtrusionVertex {
    top: DVec3,
    top_altitude: f64,
    base: DVec3,
    base_altitude: f64,
}

fn push_extrusion_wall_mesh(
    ring_positions: &[ExtrusionVertex],
    is_hole: bool,
    positions: &mut Vec<[f32; 3]>,
    normals: &mut Vec<[f32; 3]>,
    uvs: &mut Vec<[f32; 2]>,
    colors: &mut Vec<[f32; 4]>,
    indices: &mut Vec<u32>,
    height_color_gradient_strength: f32,
    height_color_gradient_upper_altitude: f64,
) {
    let ring_len = ring_positions.len();
    if ring_len < 2 {
        return;
    }

    let ring_area = signed_area_xy(ring_positions);
    let outward_right = (ring_area >= 0.0) != is_hole;
    for edge_index in 0..ring_len {
        let next_edge_index = (edge_index + 1) % ring_len;
        let top_left = ring_positions[edge_index].top;
        let top_right = ring_positions[next_edge_index].top;
        let top_left_altitude = ring_positions[edge_index].top_altitude;
        let top_right_altitude = ring_positions[next_edge_index].top_altitude;
        let edge = top_right - top_left;
        let normal = if outward_right {
            DVec3::new(edge.y, -edge.x, 0.0)
        } else {
            DVec3::new(-edge.y, edge.x, 0.0)
        }
        .normalize_or_zero()
        .as_vec3()
        .to_array();

        let base_left = ring_positions[edge_index].base;
        let base_right = ring_positions[next_edge_index].base;
        let base_left_altitude = ring_positions[edge_index].base_altitude;
        let base_right_altitude = ring_positions[next_edge_index].base_altitude;

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
        colors.extend([
            height_gradient_color(
                top_left_altitude,
                height_color_gradient_strength,
                height_color_gradient_upper_altitude,
            ),
            height_gradient_color(
                top_right_altitude,
                height_color_gradient_strength,
                height_color_gradient_upper_altitude,
            ),
            height_gradient_color(
                base_right_altitude,
                height_color_gradient_strength,
                height_color_gradient_upper_altitude,
            ),
            height_gradient_color(
                base_left_altitude,
                height_color_gradient_strength,
                height_color_gradient_upper_altitude,
            ),
        ]);

        if outward_right {
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

fn edge_distance_image(resolution: UVec2, data: &[f32]) -> Image {
    let format = TextureFormat::R32Float;
    let mut bytes = Vec::with_capacity(std::mem::size_of_val(data));
    for value in data {
        bytes.extend_from_slice(&value.to_ne_bytes());
    }

    let mut image = Image::new(
        Extent3d {
            width: resolution.x,
            height: resolution.y,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        bytes,
        format,
        RenderAssetUsages::default(),
    );
    image.sampler = ImageSampler::linear();
    image
}

fn calculate_edge_distance_texture(
    bounds: (bevy::math::DVec2, bevy::math::DVec2),
    features: &HashMap<String, SourceLayerFeature>,
    edge_texture: &mut FeatureTileBucketEdgeDistanceTexture,
) {
    edge_texture.rasterized_feature_ids.clear();
    edge_texture
        .rasterized_feature_ids
        .extend(features.keys().cloned());

    let edges = edge_segments_uv(bounds, features);
    update_edge_distance_texture(
        &edges,
        &mut edge_texture.data,
        edge_texture.resolution.x as usize,
        edge_texture.resolution.y as usize,
        EDGE_DISTANCE_MAX_UV,
    );
}

fn edge_segments_uv(
    bounds: (bevy::math::DVec2, bevy::math::DVec2),
    features: &HashMap<String, SourceLayerFeature>,
) -> Vec<f32> {
    let mut edges = Vec::new();
    for feature in features.values() {
        match &feature.geometry.value {
            Value::Polygon(polygon) => append_polygon_edge_segments_uv(bounds, polygon, &mut edges),
            Value::MultiPolygon(polygons) => {
                for polygon in polygons {
                    append_polygon_edge_segments_uv(bounds, polygon, &mut edges);
                }
            }
            _ => {}
        }
    }
    edges
}

fn append_polygon_edge_segments_uv(
    bounds: (bevy::math::DVec2, bevy::math::DVec2),
    polygon: &[Vec<Vec<f64>>],
    edges: &mut Vec<f32>,
) {
    for ring in polygon {
        let ring_positions = ring_without_closing_position(ring);
        let uvs = ring_positions
            .iter()
            .filter(|&position| position.len() >= 2)
            .map(|position| tile_uv(bounds, dvec2(position[0], position[1])))
            .collect::<Vec<_>>();

        if uvs.len() < 2 {
            continue;
        }

        for index in 0..uvs.len() {
            let a = uvs[index];
            let b = uvs[(index + 1) % uvs.len()];
            edges.extend([a.x, a.y, b.x, b.y]);
        }
    }
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
