use crate::app::common::settings::Settings;
use crate::utils::mercator_coordinate::{LngLat, MercatorCoordinate};
use bevy::asset::RenderAssetUsages;
use bevy::camera::{
    CameraProjection, RenderTarget,
    visibility::{NoFrustumCulling, RenderLayers},
};
use bevy::math::{DMat4, DQuat, DVec2, DVec3, DVec4, dvec3};
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::window::WindowRef;
use big_space::prelude::{CellCoord, FloatingOrigin, Grid};
use serde::Deserialize;
use std::collections::{HashMap, HashSet};

pub const MERCATOR_WORLD_SIZE: f64 = 100_000.0;
const MAPLIBRE_DEFAULT_FOV_RADIANS: f64 = 0.643_501_108_793_284_4;
const TILE_DEBUG_BORDER_Z: f32 = 0.05;

pub struct MapViewCorePlugin;

impl Plugin for MapViewCorePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, configure_map_view_debug_gizmos);

        app.add_systems(
            Update,
            (
                sync_map_view_cameras,
                sync_map_view_tile_managers,
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

#[derive(Component, Default)]
pub struct MapViewTileManager {
    pub map_view: Option<Entity>,
    pub active_tiles: Vec<Tile>,
    pub tiles: HashMap<TileKey, (Entity, MapViewTile)>,
    pub pending_textures: HashMap<TileKey, MapViewTileTexture>,
}

#[derive(Clone, Component)]
pub struct MapViewTile {
    pub map_view: Entity,
    pub key: TileKey,
    pub material: Handle<StandardMaterial>,
    pub texture: Option<Handle<Image>>,
    pub tile: Tile,
}

pub struct MapViewTileTexture {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
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

fn sync_map_view_tile_managers(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    _settings: Option<Res<Settings>>,
    map_views: Query<(&Grid, &MapView)>,
    mut managers: Query<&mut MapViewTileManager>,
) {
    for mut manager in &mut managers {
        let Some(map_view_id) = manager.map_view else {
            continue;
        };
        let Ok((map_view_grid, map_view)) = map_views.get(map_view_id) else {
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

            let (tile_pos, tile_size) = tile_transform_d(&synced_tile, 0.);
            let (tile_cell, tile_cell_pos) = map_view_grid.translation_to_grid(tile_pos);

            if let Some((tile_id, tile)) = manager.tiles.get_mut(&key) {
                tile.tile = synced_tile.clone();
                commands
                    .entity(*tile_id)
                    .insert((tile_cell, Transform::from_translation(tile_cell_pos)));
                continue;
            }

            let material = materials.add(StandardMaterial {
                base_color: tile_color(key).with_alpha(0.25),
                unlit: true,
                ..default()
            });

            let tile = MapViewTile {
                map_view: map_view_id,
                key,
                tile: synced_tile.clone(),
                material: material.clone(),
                texture: None,
            };

            let tile_id = commands
                .spawn((
                    Name::new(format!("Tile {key:?}")),
                    tile_cell,
                    Transform::from_translation(tile_cell_pos),
                    Mesh3d(meshes.add(Rectangle::new(tile_size.x as f32, tile_size.y as f32))),
                    MeshMaterial3d(material),
                    RenderLayers::layer(map_view.render_layer),
                    NoFrustumCulling,
                    tile.clone(),
                ))
                .id();

            commands.entity(map_view_id).add_child(tile_id);

            manager.tiles.insert(key, (tile_id, tile));
        }

        let pending_textures = manager.pending_textures.drain().collect::<Vec<_>>();
        for (key, texture) in pending_textures {
            info!("Receiving tile texture for {key:?}");
            let Some((tile_id, tile)) = manager.tiles.get_mut(&key) else {
                continue;
            };

            if texture.width == 0
                || texture.height == 0
                || texture.rgba.len() != (texture.width as usize * texture.height as usize * 4)
            {
                continue;
            }

            let image = Image::new(
                Extent3d {
                    width: texture.width,
                    height: texture.height,
                    depth_or_array_layers: 1,
                },
                TextureDimension::D2,
                texture.rgba,
                TextureFormat::Rgba8UnormSrgb,
                RenderAssetUsages::default(),
            );
            let image_handle = images.add(image);

            if let Some(material) = materials.get_mut(&tile.material) {
                material.base_color = Color::WHITE;
                material.base_color_texture = Some(image_handle.clone());
            }

            tile.texture = Some(image_handle.clone());
            commands.entity(*tile_id).insert(tile.clone());
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

pub fn spawn_map_view_tile_manager(commands: &mut Commands, map_view: Entity) -> Entity {
    commands
        .spawn(MapViewTileManager {
            map_view: Some(map_view),
            active_tiles: Vec::new(),
            tiles: HashMap::new(),
            pending_textures: HashMap::new(),
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
        DVec4::new(0.0, 0.0, 0.5, 0.0),
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
