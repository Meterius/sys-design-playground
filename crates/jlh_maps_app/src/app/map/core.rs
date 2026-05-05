use bevy::camera::{visibility::RenderLayers, RenderTarget, ScalingMode};
use bevy::prelude::*;
use bevy::window::WindowRef;
use std::collections::{HashMap, HashSet};

pub const MERCATOR_WORLD_SIZE: f32 = 100_000.0;
const MAPLIBRE_TILE_SIZE: f32 = 512.0;
const TOP_DOWN_CAMERA_Z: f32 = 10_000.0;

pub struct MapViewCorePlugin;

impl Plugin for MapViewCorePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (sync_map_view_cameras, sync_map_view_tile_managers).chain(),
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
    pub width: f32,
    pub height: f32,
    pub zoom: f32,
    pub pitch: f32,
    pub bearing: f32,
    pub center_lng: f32,
    pub center_lat: f32,
    pub main_matrix: Vec<f32>,
}

#[derive(Component, Default)]
pub struct MapViewTileManager {
    pub map_view: Option<Entity>,
    pub active_tile_keys: Vec<TileKey>,
    pub tiles: HashMap<TileKey, MapViewTile>,
}

#[derive(Clone, Component)]
pub struct MapViewTile {
    pub map_view: Entity,
    pub key: TileKey,
    pub entity: Entity,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct TileKey {
    pub z: u32,
    pub x: u32,
    pub y: u32,
}

fn sync_map_view_cameras(
    mut query: Query<(&MapViewCamera, &mut Transform, &mut Projection)>,
) {
    for (camera, mut transform, mut projection) in &mut query {
        let center = lng_lat_to_mercator_world(camera.state.center_lng, camera.state.center_lat);
        *transform = top_down_camera_transform(center);

        if let Projection::Orthographic(orthographic) = projection.as_mut() {
            let zoom_scale = MAPLIBRE_TILE_SIZE * 2.0_f32.powf(camera.state.zoom.max(0.0));
            let viewport_height = if camera.state.height > 0.0 && zoom_scale > 0.0 {
                (camera.state.height / zoom_scale) * MERCATOR_WORLD_SIZE
            } else {
                MERCATOR_WORLD_SIZE
            };

            orthographic.scaling_mode = ScalingMode::FixedVertical { viewport_height };
        }
    }
}

fn sync_map_view_tile_managers(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    map_views: Query<&MapView>,
    mut managers: Query<&mut MapViewTileManager>,
) {
    for mut manager in &mut managers {
        let Some(map_view_entity) = manager.map_view else {
            continue;
        };
        let Ok(map_view) = map_views.get(map_view_entity) else {
            continue;
        };

        let active_tiles = manager
            .active_tile_keys
            .iter()
            .copied()
            .collect::<HashSet<_>>();

        manager.tiles.retain(|key, tile| {
            let keep = active_tiles.contains(key);
            if !keep {
                commands.entity(tile.entity).despawn();
            }
            keep
        });

        for key in manager.active_tile_keys.clone() {
            if manager.tiles.contains_key(&key) {
                continue;
            }

            let entity = commands.spawn_empty().id();
            let tile = MapViewTile {
                map_view: map_view_entity,
                key,
                entity,
            };
            let (center, size) = tile_transform(key);

            commands.entity(entity).insert((
                Mesh3d(meshes.add(Rectangle::new(1.0, 1.0))),
                MeshMaterial3d(materials.add(StandardMaterial {
                    base_color: tile_color(key),
                    unlit: true,
                    ..default()
                })),
                Transform::from_translation(center).with_scale(Vec3::splat(size)),
                RenderLayers::layer(map_view.render_layer),
                tile.clone(),
            ));

            manager.tiles.insert(key, tile);
        }
    }
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
                    viewport_height: MERCATOR_WORLD_SIZE,
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
            active_tile_keys: Vec::new(),
            tiles: HashMap::new(),
        })
        .id()
}

fn tile_transform(key: TileKey) -> (Vec3, f32) {
    let scale = 2.0_f32.powi(key.z as i32);
    let size = MERCATOR_WORLD_SIZE / scale;
    let mercator_x = ((key.x as f32 + 0.5) / scale) * MERCATOR_WORLD_SIZE;
    let y = ((key.y as f32 + 0.5) / scale) * MERCATOR_WORLD_SIZE;

    (Vec3::new(mercator_x_to_world_x(mercator_x), y, 0.0), size)
}

fn top_down_camera_transform(center: Vec3) -> Transform {
    Transform::from_translation(center + Vec3::Z * TOP_DOWN_CAMERA_Z)
        .looking_to(Vec3::NEG_Z, Vec3::NEG_Y)
}

fn lng_lat_to_mercator_world(lng: f32, lat: f32) -> Vec3 {
    let mercator_x = ((lng + 180.0) / 360.0) * MERCATOR_WORLD_SIZE;
    let clamped_lat = lat.clamp(-85.051_13, 85.051_13);
    let lat_rad = clamped_lat.to_radians();
    let y =
        (1.0 - ((lat_rad.tan() + (1.0 / lat_rad.cos())).ln() / std::f32::consts::PI)) * 0.5;

    Vec3::new(
        mercator_x_to_world_x(mercator_x),
        y * MERCATOR_WORLD_SIZE,
        0.0,
    )
}

fn mercator_x_to_world_x(x: f32) -> f32 {
    MERCATOR_WORLD_SIZE - x
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
