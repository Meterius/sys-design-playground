use crate::app::geo::{GeoMapElementOf, GeoMapPlane};
use crate::geo::coords::{Projection2D, RadLonLatVec2};
use crate::geo::sub_division::{SubDivision2d, TileKey};
use crate::geo::tiling::{TileServer, TileServerError};
use bevy::ecs::system::RunSystemOnce;
use bevy::math::USizeVec2;
use bevy::prelude::*;
use bevy_tokio_tasks::TokioTasksRuntime;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

#[derive(Default)]
pub struct GeoMapTilingPlugin {}

impl Plugin for GeoMapTilingPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, startup);
        app.add_systems(
            Update,
            (
                geo_map_plane_tiling_tile_management,
                geo_map_plane_tiling_update_sprite,
            )
                .chain(),
        );
    }
}

fn startup(world: &mut World) {
    world.insert_non_send_resource(TileServer::new(PathBuf::from_iter(["assets", "cache"])));

    world.insert_non_send_resource(GeoMapTileImageStore {
        tiles: HashMap::new(),
        requested: HashSet::new(),
        failed: HashSet::new(),
    });
}

pub struct GeoMapTileImageStore {
    requested: HashSet<TileKey>,
    failed: HashSet<TileKey>,
    tiles: HashMap<TileKey, Handle<Image>>,
}

impl GeoMapTileImageStore {
    const MAX_INFLIGHT_COUNT: usize = 32;
}

fn geo_map_plane_tiling_update_sprite(
    mut commands: Commands,
    mut store: NonSendMut<GeoMapTileImageStore>,
    tile_server: NonSend<TileServer>,
    runtime: Res<TokioTasksRuntime>,
    tiles_without_sprite: Query<(Entity, &GeoMapElementOf, &GeoMapPlaneTile), Without<Sprite>>,
    planes: Query<&GeoMapPlane>,
) {
    struct RequestedTile {
        priority: f32,
        tile: TileKey,
    }

    let mut requests = Vec::new();

    for (tile_id, tile_element_of, tile) in tiles_without_sprite {
        if let Ok(plane) = planes.get(tile_element_of.0) {
            if let Some(handle) = store.tiles.get(&tile.key) {
                commands.entity(tile_id).insert(Sprite {
                    image: handle.clone(),
                    custom_size: Some(plane.scale * (tile.bb_abs.1 - tile.bb_abs.0)),
                    ..default()
                });
            } else if !store.requested.contains(&tile.key)
                && !store.failed.contains(&tile.key)
                && tile.targeted
            {
                requests.push((tile.key.clone(), plane.projection.clone()));
            }
        }
    }

    requests.sort_by_key(|(key, _)| key.len());

    for (requested_tile, projection) in requests.into_iter().take(
        GeoMapTileImageStore::MAX_INFLIGHT_COUNT - store.requested.len(),
    ) {
        store.requested.insert(requested_tile.clone());
        let tile_server = tile_server.clone();
        runtime.spawn_background_task(async move |mut task| {
            let projection = projection;
            let requested_tile = requested_tile;

            let tile_path = tile_server
                .load_tile(&projection, &requested_tile)
                .await
                .inspect_err(|err| error!("{err}"));

            task.run_on_main_thread(|ctx| {
                let _ = ctx
                    .world
                    .run_system_once_with(
                        |(In(tile), In(tile_path)): (
                            In<TileKey>,
                            In<Result<PathBuf, TileServerError>>,
                        ),
                         asset_server: Res<AssetServer>,
                         mut store: NonSendMut<GeoMapTileImageStore>| {
                            store.requested.remove(&tile);

                            if let Ok(tile_path) = tile_path {
                                store.tiles.insert(tile, asset_server.load(PathBuf::from_iter(tile_path.iter().skip(1))));
                            } else {
                                store.failed.insert(tile);
                            }
                        },
                        (requested_tile, tile_path),
                    )
                    .inspect_err(|err| error!("{err}"));
            })
            .await;
        });
    }
}

#[derive(Component)]
pub struct GeoMapPlaneTiling {
    pub target_count: usize,
    tile_map: HashMap<TileKey, Entity>,
}

impl GeoMapPlaneTiling {
    pub fn new(target_count: usize) -> Self {
        Self {
            target_count,
            tile_map: HashMap::new(),
        }
    }
}

#[derive(Component)]
pub struct GeoMapPlaneTile {
    pub key: TileKey,
    pub bb_gcs: (RadLonLatVec2, RadLonLatVec2),
    pub bb_abs: (Vec2, Vec2),
    pub targeted: bool,
}

fn geo_map_plane_tiling_tile_management(
    mut commands: Commands,
    tiling: Query<(&mut GeoMapPlaneTiling, &GeoMapElementOf)>,
    planes: Query<(Entity, &GlobalTransform, &GeoMapPlane)>,
    camera: Query<(&GlobalTransform, &Camera)>,
    mut tiles: Query<&mut GeoMapPlaneTile>,
) {
    if let Ok((camera_transform, camera)) = camera.single() {
        for (mut tiling, tiling_element_of) in tiling {
            if let Ok((plane_id, plane_transform, plane)) = planes.get(tiling_element_of.0) {
                let plane_pos = plane_transform.translation().xy();

                let cam_global_min = camera
                    .ndc_to_world(camera_transform, Vec2::NEG_ONE.extend(0.0))
                    .map(Vec3::xy);
                let cam_global_max = camera
                    .ndc_to_world(camera_transform, Vec2::ONE.extend(0.0))
                    .map(Vec3::xy);

                if let Some(cam_global_min) = cam_global_min
                    && let Some(cam_global_max) = cam_global_max
                {
                    let cam_abs_min = plane.local_to_abs(&(cam_global_min - plane_pos));
                    let cam_abs_max = plane.local_to_abs(&(cam_global_max - plane_pos));

                    let cam_abs_bbox = (cam_abs_min, cam_abs_max);

                    let sub_division = SubDivision2d::from_corners(
                        plane.projection.abs_pos() - 0.5 * plane.projection.abs_size(),
                        plane.projection.abs_pos() + 0.5 * plane.projection.abs_size(),
                    );

                    let target_depth = sub_division.min_depth_for_tile_count(
                        cam_abs_bbox,
                        USizeVec2::new(tiling.target_count, tiling.target_count),
                    );

                    let mut untargeted_tiles = tiling.tile_map.clone();

                    for depth in 0..=target_depth.min(9) {
                        for tile in sub_division.tile_covering(cam_abs_bbox, depth) {
                        untargeted_tiles.remove(&tile.key);

                            if let std::collections::hash_map::Entry::Vacant(e) =
                                tiling.tile_map.entry(tile.key.clone())
                            {
                                let bb_gcs = (
                                    plane.projection.abs_to_gcs(&tile.bb_min),
                                    plane.projection.abs_to_gcs(&tile.bb_max),
                                );

                                let tile_id = commands
                                    .entity(plane_id)
                                    .with_child((
                                        GeoMapElementOf(plane_id),
                                        GeoMapPlaneTile {
                                            key: tile.key.clone(),
                                            bb_abs: (tile.bb_min, tile.bb_max),
                                            bb_gcs,
                                            targeted: true,
                                        },
                                        Transform::from_translation(
                                            plane
                                                .abs_to_local(&((tile.bb_min + tile.bb_max) / 2.0))
                                                .extend(1.0 + tile.key.len() as f32 * 0.0001),
                                        ),
                                        Visibility::default(),
                                    ))
                                    .id();

                                e.insert(tile_id);
                            }
                        }
                    }

                    for &tile_id in tiling.tile_map.values() {
                        if let Ok(mut tile) = tiles.get_mut(tile_id) {
                            let targeted = !untargeted_tiles.contains_key(&tile.key);
                            if targeted != tile.targeted {
                                tile.targeted = targeted;
                            }
                        }
                    }
                }
            }
        }
    }
}
