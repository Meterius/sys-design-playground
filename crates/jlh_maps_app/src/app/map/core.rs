use bevy::asset::RenderAssetUsages;
use bevy::camera::{
    CameraProjection, RenderTarget,
    visibility::{NoFrustumCulling, RenderLayers},
};
use bevy::math::{DVec2, DVec3, dvec3, DMat4, DVec4};
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::ui::UiTargetCamera;
use bevy::window::WindowRef;
use serde::Deserialize;
use std::collections::{HashMap, HashSet};

use crate::app::common::settings::Settings;
use crate::utils::mercator_coordinate::{LngLat, MercatorCoordinate};

pub const MERCATOR_WORLD_SIZE: f64 = 100_000.0;
const TILE_DEBUG_BORDER_Z: f32 = 0.05;
const TILE_DEBUG_LABEL_INSET: f32 = 4.0;

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
                sync_map_view_tile_debug_labels,
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
    config.render_layers =
        RenderLayers::from_layers(&(0..32).collect::<Vec<_>>());
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
    pub tiles: HashMap<TileKey, MapViewTile>,
    pub pending_textures: HashMap<TileKey, MapViewTileTexture>,
}

#[derive(Clone, Component)]
pub struct MapViewTile {
    pub map_view: Entity,
    pub key: TileKey,
    pub entity: Entity,
    pub material: Handle<StandardMaterial>,
    pub texture: Option<Handle<Image>>,
    pub debug_label: Option<Entity>,
    pub tile: Tile,
}

#[derive(Component)]
struct MapViewTileDebugLabel {
    map_view: Entity,
    tile: Tile,
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

fn sync_map_view_cameras(mut query: Query<(&MapViewCamera, &mut Transform, &mut Projection)>) {
    for (camera, mut transform, mut projection) in &mut query {
        *transform = Transform::IDENTITY;

        let Some(maplibre_projection) =
            MapLibreMercatorProjection::from_main_matrix(&camera.state.main_matrix)
        else {
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
    settings: Option<Res<Settings>>,
    map_views: Query<&MapView>,
    cameras: Query<(Entity, &MapViewCamera)>,
    mut managers: Query<&mut MapViewTileManager>,
) {
    let debug_visibility = if Settings::in_debug_mode(settings) {
        Visibility::Visible
    } else {
        Visibility::Hidden
    };

    for mut manager in &mut managers {
        let Some(map_view_entity) = manager.map_view else {
            continue;
        };
        let Ok(map_view) = map_views.get(map_view_entity) else {
            continue;
        };

        let active_tiles = manager
            .active_tiles
            .iter()
            .map(|tile| tile.key)
            .collect::<HashSet<_>>();

        manager.tiles.retain(|key, tile| {
            let keep = active_tiles.contains(key);
            if !keep {
                commands.entity(tile.entity).despawn();
                if let Some(label_entity) = tile.debug_label {
                    commands.entity(label_entity).despawn();
                }
            }
            keep
        });

        for synced_tile in manager.active_tiles.clone() {
            let key = synced_tile.key;
            if manager.tiles.contains_key(&key) {
                continue;
            }

            let entity = commands.spawn(Name::new(format!("Tile {key:?}"))).id();
            let material = materials.add(StandardMaterial {
                base_color: tile_color(key).with_alpha(0.25),
                unlit: true,
                ..default()
            });
            let tile = MapViewTile {
                map_view: map_view_entity,
                key,
                tile: synced_tile.clone(),
                entity,
                material: material.clone(),
                texture: None,
                debug_label: None,
            };
            let (center, size) = tile_transform(&synced_tile, 0.);

            commands.entity(entity).insert((
                Mesh3d(meshes.add(Rectangle::new(1.0, 1.0))),
                MeshMaterial3d(material),
                Transform::from_translation(center).with_scale(size.extend(1.0)),
                RenderLayers::layer(map_view.render_layer),
                NoFrustumCulling,
                tile.clone(),
            ));

            let debug_label = spawn_tile_debug_label(
                &mut commands,
                cameras.iter().find_map(|(camera_entity, camera)| {
                    (camera.map_view == Some(map_view_entity)).then_some(camera_entity)
                }),
                map_view_entity,
                synced_tile.clone(),
                debug_visibility,
            );

            let mut tile = tile;
            tile.debug_label = Some(debug_label);
            commands.entity(entity).insert(tile.clone());
            manager.tiles.insert(key, tile);
        }

        for tile in manager.tiles.values() {
            if let Some(label_entity) = tile.debug_label {
                commands.entity(label_entity).insert(debug_visibility);
            }
        }

        let pending_textures = manager.pending_textures.drain().collect::<Vec<_>>();
        for (key, texture) in pending_textures {
            info!("Receiving tile texture for {key:?}");
            let Some(tile) = manager.tiles.get_mut(&key) else {
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
            commands.entity(tile.entity).insert(tile.clone());
        }
    }
}

fn draw_map_view_tile_debug_gizmos(
    mut gizmos: Gizmos,
    settings: Option<Res<Settings>>,
    managers: Query<&MapViewTileManager>,
) {
    if !Settings::in_debug_mode(settings) {
        return;
    }

    for manager in &managers {
        for tile in manager.tiles.values() {
            let (center, size) = tile_transform(&tile.tile, 0.0);
            gizmos.rect(
                Isometry3d::from_translation(center.with_z(TILE_DEBUG_BORDER_Z)),
                size,
                Color::srgb(1.0, 0.0, 0.0),
            );
        }
    }
}

fn sync_map_view_tile_debug_labels(
    mut commands: Commands,
    settings: Option<Res<Settings>>,
    cameras: Query<(Entity, &Camera, &GlobalTransform, &MapViewCamera)>,
    mut labels: Query<(Entity, &MapViewTileDebugLabel, &mut Node, &mut Visibility)>,
) {
    let debug_enabled = Settings::in_debug_mode(settings);

    for (label_entity, label, mut node, mut visibility) in &mut labels {
        if !debug_enabled {
            *visibility = Visibility::Hidden;
            continue;
        }

        let Some((camera_entity, camera, camera_transform, _)) = cameras
            .iter()
            .find(|(_, _, _, camera)| camera.map_view == Some(label.map_view))
        else {
            *visibility = Visibility::Hidden;
            continue;
        };

        let (center, size) = tile_transform(&label.tile, 0.);
        let top_left = Vec3::new(center.x - size.x * 0.5, center.y + size.y * 0.5, center.z);
        let Ok(viewport_position) = camera.world_to_viewport(camera_transform, top_left) else {
            *visibility = Visibility::Hidden;
            continue;
        };

        commands
            .entity(label_entity)
            .insert(UiTargetCamera(camera_entity));
        node.left = Val::Px(viewport_position.x + TILE_DEBUG_LABEL_INSET);
        node.top = Val::Px(viewport_position.y + TILE_DEBUG_LABEL_INSET);
        *visibility = Visibility::Visible;
    }
}

fn spawn_tile_debug_label(
    commands: &mut Commands,
    camera: Option<Entity>,
    map_view: Entity,
    tile: Tile,
    visibility: Visibility,
) -> Entity {
    let key = tile.key.clone();

    let mut entity = commands.spawn((
        Name::new(format!("Tile {key:?} debug label")),
        Text::new(format!("{}/{}/{}", key.z, key.x, key.y)),
        TextFont::from_font_size(12.0),
        TextColor(Color::WHITE),
        TextLayout::new_with_justify(Justify::Left),
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(0.0),
            top: Val::Px(0.0),
            padding: UiRect::axes(Val::Px(3.0), Val::Px(1.0)),
            ..default()
        },
        BackgroundColor(Color::BLACK.with_alpha(0.75)),
        MapViewTileDebugLabel { map_view, tile },
        visibility,
    ));

    if let Some(camera) = camera {
        entity.insert(UiTargetCamera(camera));
    }

    entity.id()
}

pub fn spawn_map_view_camera(
    commands: &mut Commands,
    map_view: Entity,
    render_layer: usize,
) -> Entity {
    commands
        .spawn((
            Camera3d::default(),
            Projection::custom(MapLibreMercatorProjection::default()),
            Transform::IDENTITY,
            RenderTarget::Window(WindowRef::Entity(map_view)),
            RenderLayers::layer(render_layer),
            MapViewCamera {
                map_view: Some(map_view),
                state: MapViewCameraState::default(),
            },
        ))
        .id()
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

fn tile_transform(tile: &Tile, alt: f64) -> (Vec3, Vec2) {
    let south_west = lng_lat_to_world(tile.bounds_lnglat.0.x, tile.bounds_lnglat.0.y, alt);
    let north_east = lng_lat_to_world(tile.bounds_lnglat.1.x, tile.bounds_lnglat.1.y, alt);

    let min = south_west.min(north_east);
    let max = south_west.max(north_east);
    let size = max - min;

    ((min + max) * 0.5, size.xy())
}

fn lng_lat_to_world(lng: f64, lat: f64, alt: f64) -> Vec3 {
    mercator_to_world(MercatorCoordinate::from_lng_lat(LngLat::new(lng, lat), alt)).as_vec3()
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
    fn from_main_matrix(main_matrix: &[f64]) -> Option<Self> {
        let main_matrix = main_matrix.try_into().ok()?;

        let maplibre_clip_from_mercator = DMat4::from_cols_array(main_matrix);
        let mercator_from_world = DMat4::from_cols(
            DVec4::new(1.0 / MERCATOR_WORLD_SIZE, 0.0, 0.0, 0.0),
            DVec4::new(0.0, -1.0 / MERCATOR_WORLD_SIZE, 0.0, 0.0),
            DVec4::new(0.0, 0.0, 1.0 / MERCATOR_WORLD_SIZE, 0.0),
            DVec4::W,
        );

        Some(Self {
            clip_from_view: (
                opengl_to_wgpu_clip_matrix()
                * maplibre_clip_from_mercator
                * mercator_from_world
            ).as_mat4(),
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
