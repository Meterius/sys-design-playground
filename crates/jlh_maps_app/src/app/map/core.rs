use crate::app::common::settings::Settings;
use crate::utils::debug::SoftExpect;
use crate::utils::mercator_coordinate::{EARTH_CIRCUMFERENCE, LngLat, MercatorCoordinate, lng_from_mercator_x, lat_from_mercator_y};
use crate::utils::terrain::{TerrainData, get_dem_elevation};
use crate::utils::terrain_mesh::build_terrain_mesh_with_skirts;
use bevy::asset::RenderAssetUsages;
use bevy::camera::{
    CameraProjection, RenderTarget,
    visibility::{NoFrustumCulling, RenderLayers},
};
use bevy::math::{DMat4, DQuat, DVec2, DVec3, DVec4, dvec3, dvec2};
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::window::WindowRef;
use big_space::prelude::{CellCoord, FloatingOrigin, Grid};
use geojson::{Geometry, Value};
use serde::Deserialize;
use std::collections::{HashMap, HashSet};

pub const MERCATOR_WORLD_SIZE: f64 = 100_000.0;
const MAPLIBRE_DEFAULT_FOV_RADIANS: f64 = 0.643_501_108_793_284_4;
const TILE_DEBUG_BORDER_Z: f32 = 0.05;
const TILE_TERRAIN_MESH_RESOLUTION: u32 = 128;
const TILE_TERRAIN_TEXTURE_SIZE: u32 = 128;

pub struct MapViewCorePlugin;

impl Plugin for MapViewCorePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, configure_map_view_debug_gizmos);

        app.add_systems(
            Update,
            (
                sync_map_view_cameras,
                sync_map_view_feature_managers,
                sync_map_view_features,
                sync_map_view_tile_managers,
                sync_map_view_tiles,
                draw_map_view_tile_debug_gizmos.run_if(Settings::in_debug_mode),
            )
                .chain(),
        );
    }
}

fn configure_map_view_debug_gizmos(mut gizmo_config_store: ResMut<GizmoConfigStore>) {
    let (config, _) = gizmo_config_store.config_mut::<DefaultGizmoConfigGroup>();
    config.depth_bias = -1.0;
    config.line.width = 2.0;
    config.line.perspective = false;
    config.render_layers = RenderLayers::from_layers(&(0..32).collect::<Vec<_>>());
}

#[derive(Component)]
pub struct MapView {
    pub render_layer: usize,
}

#[derive(Component, Default)]
pub struct MapViewCamera {
    pub map_view: Option<Entity>,
    pub state: MapViewCameraState,
}

#[derive(Default)]
#[allow(dead_code)]
pub struct MapViewCameraState {
    pub width: f64,
    pub height: f64,
    pub zoom: f64,
    pub pitch: f64,
    pub bearing: f64,
    pub center_lng: f64,
    pub center_lat: f64,
    pub main_matrix: Vec<f64>,
}

#[derive(Component)]
pub struct MapViewTileManager {
    pub map_view_id: Entity,

    pub active_tiles: Vec<Tile>,
    pub tiles: HashMap<TileKey, (Entity, MapViewTile)>,

    pub terrain_data: HashMap<TileKey, MapViewTileTerrainData>,
}

#[derive(Component)]
pub struct MapViewFeatureManager {
    pub map_view_id: Entity,
    pub map_view_tile_manager_id: Entity,
    pub active_features: Vec<MapViewFeatureData>,
    pub features: HashMap<MapViewFeatureKey, (Entity, MapViewFeature)>,
}

#[derive(Clone, Component)]
pub struct MapViewTile {
    pub map_view_id: Entity,
    pub key: TileKey,
    pub material: Handle<StandardMaterial>,
    pub texture: Option<Handle<Image>>,
    pub tile: Tile,
    pub manager_id: Entity,
    pub last_synced_terrain_content_stamp: Option<String>,
    pub use_elevation_as_texture: bool,
}

#[derive(Clone, Component)]
pub struct MapViewFeature {
    pub map_view_id: Entity,
    pub manager_id: Entity,
    pub key: MapViewFeatureKey,
    pub layer_id: String,
    pub id: Option<String>,
    pub geometry: Geometry,
    pub properties: serde_json::Value,
}

#[derive(Clone, Component, Default)]
pub struct MapViewFeaturePolygonMesh {
    mesh_stamp: Option<String>,
    last_synced_terrain_content_stamp: Option<String>,
}

pub struct MapViewTileTexture {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

#[derive(Clone, Debug)]
pub struct MapViewTileTerrainData {
    pub content_stamp: String,
    pub terrain_data: TerrainData,
}

#[derive(Clone, Debug)]
pub struct MapViewFeatureData {
    pub key: MapViewFeatureKey,
    pub layer_id: String,
    pub id: Option<String>,
    pub geometry: Geometry,
    pub properties: serde_json::Value,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct MapViewFeatureKey {
    pub layer_id: String,
    pub id: String,
    pub tile_key: TileKey,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq)]
pub struct TileKey {
    pub z: u32,
    pub x: u32,
    pub y: u32,
}

#[derive(Clone, Debug)]
pub struct Tile {
    pub key: TileKey,
    pub bounds_lnglat: (DVec2, DVec2),
}

fn sync_map_view_cameras(
    map_views: Query<(&Grid, &MapView)>,
    mut query: Query<(
        &MapViewCamera,
        &mut CellCoord,
        &mut Transform,
        &mut Projection,
    )>,
) {
    for (camera, mut cell, mut transform, mut projection) in &mut query {
        let Some(map_view_id) = camera.map_view else {
            continue;
        };
        let Ok((map_view_grid, _map_view)) = map_views.get(map_view_id) else {
            continue;
        };

        let semantic_camera = maplibre_semantic_camera_transform(&camera.state);
        let (camera_cell, camera_translation) =
            map_view_grid.translation_to_grid(semantic_camera.translation);

        *cell = camera_cell;
        *transform = Transform::from_translation(camera_translation)
            .with_rotation(semantic_camera.rotation.as_quat());

        let Some(maplibre_projection) = MapLibreMercatorProjection::from_main_matrix(
            &camera.state.main_matrix,
            semantic_camera.world_from_view,
        ) else {
            continue;
        };

        match projection.as_mut() {
            Projection::Custom(custom) => {
                if let Some(existing) = custom.get_mut::<MapLibreMercatorProjection>() {
                    *existing = maplibre_projection;
                } else {
                    *projection = Projection::custom(maplibre_projection);
                }
            }
            _ => {
                *projection = Projection::custom(maplibre_projection);
            }
        }
    }
}

fn sync_map_view_tiles(
    map_views: Query<(&Grid, &MapView)>,
    mut managers: Query<&mut MapViewTileManager>,
    mut tiles: Query<(
        &mut MapViewTile,
        &mut Transform,
        &mut CellCoord,
        &mut Mesh3d,
    )>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for (mut tile, mut tile_transform, mut tile_cell, mut tile_mesh) in tiles.iter_mut() {
        if let Some(mut manager) = managers.get_mut(tile.manager_id).ok().soft_expect("")
            && let Some((view_grid, _)) = map_views.get(tile.map_view_id).ok().soft_expect("")
        {
            let (tile_pos, tile_size) = tile_transform_d(&tile.tile, 0.);
            let (new_tile_cell, new_tile_cell_pos) = view_grid.translation_to_grid(tile_pos);
            let new_tile_cell_transform = Transform::from_translation(new_tile_cell_pos)
                .with_scale(tile_size.as_vec2().extend(1.0));

            *tile_transform = new_tile_cell_transform;
            *tile_cell = new_tile_cell;

            if let Some(terrain_data) = manager.terrain_data.get(&tile.key)
                && tile.last_synced_terrain_content_stamp.as_ref() != Some(&terrain_data.content_stamp)
            {
                let get_elevation = |uv: Vec2| {
                    let uv = vec2(0.0, 1.0) + vec2(1.0, -1.0) * uv;

                    let lnglat = tile.tile.bounds_lnglat.0
                        + (tile.tile.bounds_lnglat.1 - tile.tile.bounds_lnglat.0) * uv.as_dvec2();

                    let dem_elev =
                        get_dem_elevation(&terrain_data.terrain_data, uv).unwrap_or(0.0) as f64;

                    (MercatorCoordinate::from_lng_lat(LngLat::new(lnglat.x, lnglat.y), dem_elev).z
                        * MERCATOR_WORLD_SIZE) as f32
                };

                let mesh_handle = meshes.add(build_terrain_mesh_with_skirts(
                    &get_elevation,
                    TILE_TERRAIN_MESH_RESOLUTION,
                    terrain_skirt_delta(&tile.tile),
                ));
                *tile_mesh = Mesh3d(mesh_handle);

                if tile.use_elevation_as_texture {
                    let texture = build_luminosity_height_texture(
                        &get_elevation,
                        TILE_TERRAIN_TEXTURE_SIZE,
                        TILE_TERRAIN_TEXTURE_SIZE,
                        Some((0.0, 1.0)),
                    );
                    let texture_handle = images.add(texture);

                    if let Some(material) = materials.get_mut(&tile.material) {
                        material.base_color = Color::WHITE;
                        material.base_color_texture = Some(texture_handle.clone());
                        material.unlit = true;
                    }

                    tile.texture = Some(texture_handle);
                }

                tile.last_synced_terrain_content_stamp = Some(terrain_data.content_stamp.clone());
            }
        }
    }
}

fn build_luminosity_height_texture(
    get_elevation: &impl Fn(Vec2) -> f32,
    width: u32,
    height: u32,
    value_range: Option<(f32, f32)>,
) -> Image {
    let mut elevations = Vec::with_capacity((width * height) as usize);

    let mut min = value_range.map(|r| r.0).unwrap_or(f32::INFINITY);
    let mut max = value_range.map(|r| r.1).unwrap_or(f32::NEG_INFINITY);

    for y in 0..height {
        for x in 0..width {
            let uv = Vec2::new(
                x as f32 / (width.saturating_sub(1).max(1)) as f32,
                y as f32 / (height.saturating_sub(1).max(1)) as f32,
            );
            let elevation = get_elevation(uv - Vec2::splat(0.5));

            if value_range.is_none() {
                min = min.min(elevation);
                max = max.max(elevation);
            }

            elevations.push(elevation);
        }
    }

    let range_length = (max - min).max(f32::EPSILON);
    let mut rgba = Vec::with_capacity((width * height * 4) as usize);
    for elevation in elevations {
        let luminosity = (((elevation - min) / range_length).clamp(0.0, 1.0) * 255.0).round() as u8;
        rgba.extend_from_slice(&[luminosity, luminosity, luminosity, 255]);
    }

    Image::new(
        Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        rgba,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    )
}

fn terrain_skirt_delta(tile: &Tile) -> f32 {
    let center = (tile.bounds_lnglat.0 + tile.bounds_lnglat.1) * 0.5;
    let frame_delta_meters = EARTH_CIRCUMFERENCE / 2.0_f64.powi(tile.key.z as i32) / 5.0;

    (MercatorCoordinate::from_lng_lat(LngLat::new(center.x, center.y), frame_delta_meters).z
        * MERCATOR_WORLD_SIZE) as f32
}

fn sync_map_view_feature_managers(
    mut commands: Commands,
    map_views: Query<&MapView>,
    mut managers: Query<(Entity, &mut MapViewFeatureManager)>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for (manager_id, mut manager) in &mut managers {
        let Ok(map_view) = map_views.get(manager.map_view_id) else {
            continue;
        };

        let active_features = manager
            .active_features
            .iter()
            .map(|feature| feature.key.clone())
            .collect::<HashSet<_>>();

        manager.features.retain(|key, (feature_id, _feature)| {
            let keep = active_features.contains(key);
            if !keep {
                commands.entity(*feature_id).despawn();
            }
            keep
        });

        for synced_feature in manager.active_features.clone() {
            if let Some((feature_id, feature)) = manager.features.get_mut(&synced_feature.key) {
                feature.layer_id.clone_from(&synced_feature.layer_id);
                feature.id.clone_from(&synced_feature.id);
                feature.geometry.clone_from(&synced_feature.geometry);
                feature.properties.clone_from(&synced_feature.properties);
                commands.entity(*feature_id).insert(feature.clone());
                continue;
            }

            let feature = MapViewFeature {
                map_view_id: manager.map_view_id,
                manager_id,
                key: synced_feature.key.clone(),
                layer_id: synced_feature.layer_id,
                id: synced_feature.id,
                geometry: synced_feature.geometry,
                properties: synced_feature.properties,
            };

            let feature_id = commands
                .spawn((
                    Transform::default(),
                    CellCoord::default(),
                    Name::new(format!(
                        "Feature {}:{}",
                        feature.key.layer_id, feature.key.id
                    )),
                    RenderLayers::layer(map_view.render_layer),
                    MapViewFeaturePolygonMesh::default(),
                    Mesh3d(Handle::default()),
                    MeshMaterial3d(materials.add(StandardMaterial {
                        base_color: Color::srgba(0.1, 0.55, 0.95, 0.45),
                        alpha_mode: AlphaMode::Opaque,
                        unlit: false,
                        ..default()
                    })),
                    NoFrustumCulling,
                    feature.clone(),
                ))
                .id();

            commands.entity(manager.map_view_id).add_child(feature_id);
            manager
                .features
                .insert(feature.key.clone(), (feature_id, feature));
        }
    }
}

fn sync_map_view_features(
    map_views: Query<&Grid, With<MapView>>,
    feature_managers: Query<&MapViewFeatureManager>,
    tile_managers: Query<&MapViewTileManager>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut features: Query<(
        &MapViewFeature,
        &mut CellCoord,
        &mut Transform,
        &mut Mesh3d,
        &mut MapViewFeaturePolygonMesh,
    )>,
) {
    for (feature, mut cell, mut transform, mut mesh_handle, mut polygon_mesh) in &mut features {
        let Ok(map_view_grid) = map_views.get(feature.map_view_id) else {
            continue;
        };

        let Ok(feature_manager) = feature_managers.get(feature.manager_id) else {
            continue;
        };

        let Ok(tile_manager) = tile_managers.get(feature_manager.map_view_tile_manager_id) else {
            continue;
        };

        let Some(terrain_data) = tile_manager.terrain_data.get(&feature.key.tile_key) else {
            continue;
        };

        let Some(center_lnglat) = geometry_center_lnglat(&feature.geometry) else {
            continue;
        };

        let mesh_stamp = serde_json::to_string(&feature.geometry).unwrap_or_default();
        if Some(&terrain_data.content_stamp) == polygon_mesh.last_synced_terrain_content_stamp.as_ref()
            && polygon_mesh.mesh_stamp.as_ref() == Some(&mesh_stamp)
        {
            continue;
        }

        let center_alt = sample_tile_dem_elevation(feature.key.tile_key, terrain_data, center_lnglat).unwrap_or(0.0);
        let center = lng_lat_to_world(center_lnglat.x, center_lnglat.y, center_alt);
        let (feature_cell, feature_translation) = map_view_grid.translation_to_grid(center.with_z(center.z + 0.001));
        *cell = feature_cell;
        *transform = Transform::from_translation(feature_translation);

        *mesh_handle = build_feature_polygon_mesh(&feature.geometry, center, feature.key.tile_key, terrain_data)
            .map(|mesh| Mesh3d(meshes.add(mesh)))
            .unwrap_or_else(|| Mesh3d(Handle::default()));
        polygon_mesh.mesh_stamp = Some(mesh_stamp);
        polygon_mesh.last_synced_terrain_content_stamp = Some(terrain_data.content_stamp.clone());
    }
}

fn sync_map_view_tile_managers(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    _settings: Option<Res<Settings>>,
    map_views: Query<&MapView>,
    mut managers: Query<(Entity, &mut MapViewTileManager)>,
) {
    for (manager_id, mut manager) in &mut managers {
        let Ok(map_view) = map_views.get(manager.map_view_id) else {
            continue;
        };

        let active_tiles = manager
            .active_tiles
            .iter()
            .map(|tile| tile.key)
            .collect::<HashSet<_>>();

        manager.tiles.retain(|key, (tile_id, _tile)| {
            let keep = active_tiles.contains(key);
            if !keep {
                commands.entity(*tile_id).despawn();
            }
            keep
        });

        for synced_tile in manager.active_tiles.clone() {
            let key = synced_tile.key;

            if manager.tiles.contains_key(&key) {
                continue;
            }

            let material = materials.add(StandardMaterial {
                base_color: Color::srgb(0.4, 0.4, 0.4), // tile_color(key),
                ..default()
            });

            let tile = MapViewTile {
                manager_id,
                map_view_id: manager.map_view_id.clone(),
                key,
                tile: synced_tile.clone(),
                material: material.clone(),
                texture: None,
                use_elevation_as_texture: false,
                last_synced_terrain_content_stamp: None,
            };

            let tile_id = commands
                .spawn((
                    Transform::default(),
                    CellCoord::default(),
                    Name::new(format!("Tile {key:?}")),
                    Mesh3d(meshes.add(Rectangle::new(1., 1.))),
                    MeshMaterial3d(material),
                    RenderLayers::layer(map_view.render_layer),
                    NoFrustumCulling,
                    tile.clone(),
                ))
                .id();

            commands.entity(manager.map_view_id).add_child(tile_id);

            manager.tiles.insert(key, (tile_id, tile));
        }
    }
}

fn draw_map_view_tile_debug_gizmos(
    mut gizmos: Gizmos,
    tiles: Query<(&MapViewTile, &GlobalTransform)>,
) {
    for (tile, transform) in &tiles {
        let (_, size) = tile_transform_d(&tile.tile, 0.0);

        let (_, rotation, translation) = transform.to_scale_rotation_translation();
        gizmos.rect(
            Isometry3d::new(
                translation.with_z(translation.z + TILE_DEBUG_BORDER_Z),
                rotation,
            ),
            size.as_vec2(),
            Color::srgb(1.0, 0.0, 0.0),
        );
    }
}

pub fn spawn_map_view_camera(
    commands: &mut Commands,
    map_view_id: Entity,
    render_layer: usize,
) -> Entity {
    let camera = commands
        .spawn((
            Camera3d::default(),
            Projection::custom(MapLibreMercatorProjection::default()),
            CellCoord::default(),
            Transform::IDENTITY,
            FloatingOrigin,
            RenderTarget::Window(WindowRef::Entity(map_view_id)),
            RenderLayers::layer(render_layer),
            MapViewCamera {
                map_view: Some(map_view_id),
                state: MapViewCameraState::default(),
            },
        ))
        .id();
    commands.entity(map_view_id).add_child(camera);
    camera
}

pub fn spawn_map_view_tile_manager(commands: &mut Commands, map_view_id: Entity) -> Entity {
    commands
        .spawn(MapViewTileManager {
            map_view_id,
            active_tiles: Vec::new(),
            tiles: HashMap::new(),
            terrain_data: HashMap::new(),
        })
        .id()
}

pub fn spawn_map_view_feature_manager(commands: &mut Commands, map_view_id: Entity, map_view_tile_manager_id: Entity) -> Entity {
    commands
        .spawn(MapViewFeatureManager {
            map_view_id: map_view_id,
            map_view_tile_manager_id,
            active_features: Vec::new(),
            features: HashMap::new(),
        })
        .id()
}

#[derive(Clone, Copy, Debug)]
struct SemanticCameraTransform {
    translation: DVec3,
    rotation: DQuat,
    world_from_view: DMat4,
}

fn maplibre_semantic_camera_transform(state: &MapViewCameraState) -> SemanticCameraTransform {
    let center = lng_lat_to_world(state.center_lng, state.center_lat, 0.0);
    let pitch = state.pitch.to_radians();
    let bearing = state.bearing.to_radians();
    let rotation = DQuat::from_rotation_z(-bearing) * DQuat::from_rotation_x(pitch);
    let forward = rotation * -DVec3::Z;
    let distance = maplibre_camera_to_center_distance_world(state.height, state.zoom);
    let translation = center - forward * distance;

    SemanticCameraTransform {
        translation,
        rotation,
        world_from_view: DMat4::from_rotation_translation(rotation, translation),
    }
}

fn maplibre_camera_to_center_distance_world(height: f64, zoom: f64) -> f64 {
    let world_size_pixels = 512.0 * 2.0_f64.powf(zoom);
    if height <= 0.0 || world_size_pixels <= 0.0 {
        return 0.0;
    }

    let camera_to_center_pixels = 0.5 * height / (MAPLIBRE_DEFAULT_FOV_RADIANS * 0.5).tan();
    camera_to_center_pixels / world_size_pixels * MERCATOR_WORLD_SIZE
}

fn tile_transform_d(tile: &Tile, alt: f64) -> (DVec3, DVec2) {
    let south_west = lng_lat_to_world(tile.bounds_lnglat.0.x, tile.bounds_lnglat.0.y, alt);
    let north_east = lng_lat_to_world(tile.bounds_lnglat.1.x, tile.bounds_lnglat.1.y, alt);

    let min = south_west.min(north_east);
    let max = south_west.max(north_east);
    let size = max - min;

    ((min + max) * 0.5, size.xy())
}

fn lng_lat_to_world(lng: f64, lat: f64, alt: f64) -> DVec3 {
    mercator_to_world(MercatorCoordinate::from_lng_lat(LngLat::new(lng, lat), alt))
}

fn mercator_to_world(coords: MercatorCoordinate) -> DVec3 {
    let MercatorCoordinate { x, y, z } = coords;
    dvec3(x, -y, z) * MERCATOR_WORLD_SIZE
}

fn find_managed_tile(manager: &MapViewTileManager, key: TileKey) -> Option<&Tile> {
    manager
        .tiles
        .get(&key)
        .map(|(_, tile)| &tile.tile)
        .or_else(|| manager.active_tiles.iter().find(|tile| tile.key == key))
}

fn get_tile_lnglat_bounds(key: TileKey) -> (DVec2, DVec2) {
    let ll_min= dvec2(
        lng_from_mercator_x(key.x as f64 / 2f64.powf(key.z as f64)),
        lat_from_mercator_y((key.y as f64 + 1.0) / 2f64.powf(key.z as f64))
    );

    let ll_max = dvec2(
        lng_from_mercator_x((key.x + 1) as f64 / 2f64.powf(key.z as f64)),
        lat_from_mercator_y(key.y as f64 / 2f64.powf(key.z as f64))
    );

    (ll_min.min(ll_max), ll_min.max(ll_max))
}

fn sample_tile_dem_elevation(
    tile_key: TileKey,
    terrain_data: &MapViewTileTerrainData,
    lnglat: DVec2,
) -> Option<f64> {
    let bounds_lnglat = get_tile_lnglat_bounds(tile_key);

    let bounds_size = bounds_lnglat.1 - bounds_lnglat.0;
    if bounds_size.x.abs() <= f64::EPSILON || bounds_size.y.abs() <= f64::EPSILON {
        return None;
    }

    let rel = (lnglat - bounds_lnglat.0) / bounds_size;
    let dem_uv = vec2(rel.x as f32, 1.0 - rel.y as f32);

    get_dem_elevation(&terrain_data.terrain_data, dem_uv).map(f64::from)
}

fn geometry_center_lnglat(geometry: &Geometry) -> Option<DVec2> {
    let mut min = DVec2::splat(f64::INFINITY);
    let mut max = DVec2::splat(f64::NEG_INFINITY);
    let mut has_position = false;

    visit_geometry_positions(geometry, &mut |position| {
        if position.len() < 2 {
            return;
        }

        let lnglat = DVec2::new(position[0], position[1]);
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
    tile_key: TileKey,
    terrain_data: &MapViewTileTerrainData,
) -> Option<Mesh> {
    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut uvs = Vec::new();
    let mut indices = Vec::new();

    match &geometry.value {
        Value::Polygon(polygon) => push_polygon_mesh(
            polygon,
            center,
            tile_key,
            terrain_data,
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
                    tile_key,
                    terrain_data,
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
    tile_key: TileKey,
    terrain_data: &MapViewTileTerrainData,
    positions: &mut Vec<[f32; 3]>,
    normals: &mut Vec<[f32; 3]>,
    uvs: &mut Vec<[f32; 2]>,
    indices: &mut Vec<u32>,
) {
    let first_vertex = positions.len() as u32;
    let mut flat_coords = Vec::new();
    let mut hole_indices = Vec::new();
    let mut vertex_count = 0usize;

    for (ring_index, ring) in polygon.iter().enumerate() {
        let ring_positions = ring_without_closing_position(ring);
        if ring_positions.len() < 3 {
            continue;
        }

        if ring_index > 0 {
            hole_indices.push(vertex_count);
        }

        for position in ring_positions {
            if position.len() < 2 {
                continue;
            }

            let lnglat = DVec2::new(position[0], position[1]);
            let altitude = sample_tile_dem_elevation(tile_key, terrain_data, lnglat).unwrap_or(0.0);
            let world = lng_lat_to_world(lnglat.x, lnglat.y, altitude) - center;
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

#[derive(Clone, Debug)]
struct MapLibreMercatorProjection {
    clip_from_view: Mat4,
}

impl Default for MapLibreMercatorProjection {
    fn default() -> Self {
        Self {
            clip_from_view: Mat4::IDENTITY,
        }
    }
}

impl MapLibreMercatorProjection {
    fn from_main_matrix(main_matrix: &[f64], world_from_view: DMat4) -> Option<Self> {
        let main_matrix = main_matrix.try_into().ok()?;

        let maplibre_clip_from_mercator = DMat4::from_cols_array(main_matrix);
        let mercator_from_world = DMat4::from_cols(
            DVec4::new(1.0 / MERCATOR_WORLD_SIZE, 0.0, 0.0, 0.0),
            DVec4::new(0.0, -1.0 / MERCATOR_WORLD_SIZE, 0.0, 0.0),
            DVec4::new(0.0, 0.0, 1.0 / MERCATOR_WORLD_SIZE, 0.0),
            DVec4::W,
        );

        Some(Self {
            clip_from_view: (opengl_to_wgpu_clip_matrix()
                * maplibre_clip_from_mercator
                * mercator_from_world
                * world_from_view)
                .as_mat4(),
        })
    }
}

impl CameraProjection for MapLibreMercatorProjection {
    fn get_clip_from_view(&self) -> Mat4 {
        self.clip_from_view
    }

    fn get_clip_from_view_for_sub(&self, _sub_view: &bevy::camera::SubCameraView) -> Mat4 {
        self.clip_from_view
    }

    fn update(&mut self, _width: f32, _height: f32) {}

    fn far(&self) -> f32 {
        f32::MAX
    }

    fn get_frustum_corners(&self, _z_near: f32, _z_far: f32) -> [Vec3A; 8] {
        let extent = MERCATOR_WORLD_SIZE as f32;
        [
            Vec3A::new(extent, -extent, 0.0),
            Vec3A::new(extent, extent, 0.0),
            Vec3A::new(-extent, extent, 0.0),
            Vec3A::new(-extent, -extent, 0.0),
            Vec3A::new(extent, -extent, extent),
            Vec3A::new(extent, extent, extent),
            Vec3A::new(-extent, extent, extent),
            Vec3A::new(-extent, -extent, extent),
        ]
    }
}

fn opengl_to_wgpu_clip_matrix() -> DMat4 {
    DMat4::from_cols(
        DVec4::X,
        DVec4::Y,
        DVec4::new(0.0, 0.0, -0.5, 0.0),
        DVec4::new(0.0, 0.0, 0.5, 1.0),
    )
}

fn tile_color(key: TileKey) -> Color {
    let hash = key
        .z
        .wrapping_mul(73_856_093)
        .wrapping_add(key.x.wrapping_mul(19_349_663))
        .wrapping_add(key.y.wrapping_mul(83_492_791));

    let r = 0.25 + ((hash & 0xff) as f32 / 255.0) * 0.5;
    let g = 0.25 + (((hash >> 8) & 0xff) as f32 / 255.0) * 0.5;
    let b = 0.25 + (((hash >> 16) & 0xff) as f32 / 255.0) * 0.5;

    Color::srgb(r, g, b)
}
