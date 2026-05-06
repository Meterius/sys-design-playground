use bevy::asset::RenderAssetUsages;
use bevy::camera::{RenderTarget, ScalingMode, visibility::RenderLayers};
use bevy::math::DVec2;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::ui::UiTargetCamera;
use bevy::window::WindowRef;
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use bevy::math::{dvec3, DVec3};
use crate::app::common::settings::Settings;
use crate::utils::mercator_coordinate::{LngLat, MercatorCoordinate};

pub const MERCATOR_WORLD_SIZE: f64 = 100_000.0;
const MAPLIBRE_TILE_SIZE: f64 = 512.0;
const TOP_DOWN_CAMERA_Z: f32 = 10_000.0;
const TILE_DEBUG_Z: f32 = 2.0;

pub struct MapViewCorePlugin;

impl Plugin for MapViewCorePlugin {
    fn build(&self, app: &mut App) {
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
        let center = MercatorCoordinate::from_lng_lat(LngLat::new(camera.state.center_lng, camera.state.center_lat), 0.0);
        let center_world = mercator_to_world(center).as_vec3();
        
        *transform = top_down_camera_transform(center_world);

        if let Projection::Orthographic(orthographic) = projection.as_mut() {
            let zoom_scale = MAPLIBRE_TILE_SIZE * 2.0_f64.powf(camera.state.zoom.max(0.0));
            let viewport_height = if camera.state.height > 0.0 && zoom_scale > 0.0 {
                (camera.state.height / zoom_scale) * MERCATOR_WORLD_SIZE 
            } else {
                MERCATOR_WORLD_SIZE 
            } as f32;

            orthographic.scaling_mode = ScalingMode::FixedVertical { viewport_height };
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

            let entity = commands
                .spawn(Name::new(format!("Tile {key:?}")))
                .id();
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
            let (center, size) = tile_transform(&synced_tile);

            commands.entity(entity).insert((
                Mesh3d(meshes.add(Rectangle::new(1.0, 1.0))),
                MeshMaterial3d(material),
                Transform::from_translation(center).with_scale(Vec3::splat(size)),
                RenderLayers::layer(map_view.render_layer),
                tile.clone(),
            ));

            let debug_label = spawn_tile_debug_label(
                &mut commands,
                cameras
                    .iter()
                    .find_map(|(camera_entity, camera)| {
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
            let (center, size) = tile_transform(&tile.tile);
            gizmos.rect(
                Isometry3d::from_translation(center.with_z(TILE_DEBUG_Z)),
                Vec2::splat(size),
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

        let Some((camera_entity, camera, camera_transform, _)) =
            cameras
                .iter()
                .find(|(_, _, _, camera)| camera.map_view == Some(label.map_view))
        else {
            *visibility = Visibility::Hidden;
            continue;
        };

        let (center, size) = tile_transform(&label.tile);
        let top_left = Vec3::new(center.x, center.y, TILE_DEBUG_Z);
        let Ok(viewport_position) = camera.world_to_viewport(camera_transform, top_left) else {
            *visibility = Visibility::Hidden;
            continue;
        };

        commands
            .entity(label_entity)
            .insert(UiTargetCamera(camera_entity));
        node.left = Val::Px(viewport_position.x);
        node.top = Val::Px(viewport_position.y);
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
        TextLayout::new_with_justify(Justify::Center),
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
            Projection::from(OrthographicProjection {
                scaling_mode: ScalingMode::FixedVertical {
                    viewport_height: MERCATOR_WORLD_SIZE as f32,
                },
                near: 0.0,
                far: TOP_DOWN_CAMERA_Z * 2.0,
                ..OrthographicProjection::default_3d()
            }),
            top_down_camera_transform(Vec3::ZERO),
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

fn tile_transform(tile: &Tile) -> (Vec3, f32) {
    let lower = MercatorCoordinate::from_lng_lat(LngLat::new(tile.bounds_lnglat.0.x, tile.bounds_lnglat.0.y), 0.0);
    let upper = MercatorCoordinate::from_lng_lat(LngLat::new(tile.bounds_lnglat.1.x, tile.bounds_lnglat.1.y), 0.0);

    let world_lower = mercator_to_world(lower).as_vec3();
    let world_upper = mercator_to_world(upper).as_vec3();

    let size = (world_upper - world_lower).length();
    (world_lower, size)
}

fn top_down_camera_transform(center: Vec3) -> Transform {
    Transform::from_translation(center + Vec3::Z * TOP_DOWN_CAMERA_Z)
        .looking_to(Vec3::NEG_Z, Vec3::Y)
}

fn mercator_to_world(coords: MercatorCoordinate) -> DVec3 {
    let MercatorCoordinate { x, y, .. } = coords;
    dvec3(x, -y, 0.0) * MERCATOR_WORLD_SIZE
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
