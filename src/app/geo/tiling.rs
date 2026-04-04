use crate::app::geo::{GeoMapElementOf, GeoMapPlane};
use crate::geo::coords::Projection2D;
use crate::geo::sub_division::{SubDivision2d, TileKey};
use crate::geo::tiling::{TileServer, TileServerDataset, TileServerError};
use bevy::ecs::system::RunSystemOnce;
use bevy::math::USizeVec2;
use bevy::prelude::*;
use bevy_tokio_tasks::TokioTasksRuntime;
use itertools::Itertools;
use ratelimit::Ratelimiter;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use tokio_rate_limit::{RateLimiter, RateLimiterConfig};

#[derive(Default)]
pub struct GeoMapTilingPlugin {}

impl Plugin for GeoMapTilingPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, startup);
        app.add_systems(
            Update,
            (
                handle_tiling_management,
                handle_tile_setup,
                handle_tile_target_sync,
                handle_tile_image_loading,
                handle_loaded_tile_image,
            )
                .chain(),
        );
    }
}

fn startup(world: &mut World) {
    world.insert_non_send_resource(TileServer::new(256, PathBuf::from_iter(["assets", "cache"])));

    world.insert_non_send_resource(TileImageStore {
        dataset_stores: HashMap::from_iter([
            (
                TileServerDataset::SenHubSentinel2L2a,
                TileImageDatasetStore::new(Ratelimiter::new(2)),
            ),
            (
                TileServerDataset::GibsLayerModisTerraCorrectedReflectanceTrueColor,
                TileImageDatasetStore::new(Ratelimiter::new(100)),
            ),
        ]),
    });
}

pub struct TileImageDatasetStore {
    limiter: Ratelimiter,
    requested: HashSet<TileKey>,
    failed: HashSet<TileKey>,
    data: HashMap<TileKey, Option<Handle<Image>>>,
}

impl TileImageDatasetStore {
    fn new(limiter: Ratelimiter) -> Self {
        Self {
            limiter,
            data: HashMap::new(),
            requested: HashSet::new(),
            failed: HashSet::new(),
        }
    }
}

pub struct TileImageStore {
    dataset_stores: HashMap<TileServerDataset, TileImageDatasetStore>,
}

#[derive(Component)]
#[require(Tiles)]
pub struct Tiling {
    pub target_count: usize,
    pub spawned_tiles: HashSet<TileKey>,
    pub targeted_tiles: HashSet<TileKey>,
}

#[derive(Default, Component)]
#[relationship_target(relationship = TileOf)]
pub struct Tiles(Vec<Entity>);

#[derive(Component)]
#[relationship(relationship_target = Tiles)]
pub struct TileOf(Entity);

impl Tiling {
    pub fn new(target_count: usize) -> Self {
        Self {
            target_count,
            spawned_tiles: HashSet::new(),
            targeted_tiles: HashSet::new(),
        }
    }
}

#[derive(Component)]
pub struct Tile {
    pub key: TileKey,
    pub bb_abs: (Vec2, Vec2),
}

#[derive(Component)]
pub struct TileTargeted;

#[derive(Component)]
pub struct TileImage {
    dataset: TileServerDataset,
}

#[derive(Component)]
pub struct TileImageLoaded {
    image_handle: Option<Handle<Image>>,
}

fn handle_loaded_tile_image(
    mut commands: Commands,
    loaded_tile_image: Query<
        (Entity, &ChildOf, &TileImageLoaded),
        (With<TileImage>, Added<TileImageLoaded>),
    >,
    tiles: Query<(&Tile, &GeoMapElementOf)>,
    planes: Query<&GeoMapPlane>,
) {
    for (tile_image_id, tile_image_child_of, tile_image_loaded) in loaded_tile_image {
        if let Some((tile, tile_element_of)) = tiles.get(tile_image_child_of.parent()).ok()
            && let Some(plane) = planes.get(tile_element_of.0).ok()
            && let Some(image_handle) = tile_image_loaded.image_handle.as_ref()
        {
            commands.entity(tile_image_id).insert(Sprite {
                image: image_handle.clone(),
                custom_size: Some(plane.scale * (tile.bb_abs.1 - tile.bb_abs.0)),
                ..default()
            });
        }
    }
}

fn handle_tile_image_loading(
    mut commands: Commands,
    mut store: NonSendMut<TileImageStore>,
    tile_server: NonSend<TileServer>,
    runtime: Res<TokioTasksRuntime>,
    targeted_tiles: Query<(&Children, &GeoMapElementOf, &Tile), With<TileTargeted>>,
    unloaded_tile_images: Query<(Entity, &TileImage), Without<TileImageLoaded>>,
    planes: Query<&GeoMapPlane>,
    asset_server: Res<AssetServer>,
) {
    let mut requests_by_dataset = HashMap::new();

    for (children, tile_element_of, tile) in targeted_tiles {
        if let Ok(plane) = planes.get(tile_element_of.0) {
            for (tile_image_id, tile_image) in children
                .iter()
                .filter_map(|child_id| unloaded_tile_images.get(child_id).ok())
            {
                if let Some(dataset_store) = store.dataset_stores.get(&tile_image.dataset) {
                    if let Some(handle) = dataset_store.data.get(&tile.key) {
                        commands.entity(tile_image_id).insert(TileImageLoaded {
                            image_handle: handle.clone(),
                        });
                    } else if dataset_store.failed.contains(&tile.key) {
                        commands
                            .entity(tile_image_id)
                            .insert(TileImageLoaded { image_handle: None });
                    } else if !dataset_store.requested.contains(&tile.key) {
                        let requests = requests_by_dataset
                            .entry(tile_image.dataset.clone())
                            .or_insert_with(|| Vec::new());
                        requests.push((tile.key.clone(), plane.projection.clone()));
                    }
                }
            }
        }
    }

    fn translate_tile_server_path(path: &PathBuf) -> PathBuf {
        PathBuf::from_iter(path.iter().skip(1))
    }

    for (dataset, mut requests) in requests_by_dataset.into_iter() {
        if let Some(dataset_store) = store.dataset_stores.get_mut(&dataset) {
            requests.sort_by_key(|(key, _)| key.len());

            for (request_tile_key, request_projection) in requests.into_iter() {
                match tile_server.load_tile_offline_blocking(
                    dataset.clone(),
                    &request_projection,
                    &request_tile_key,
                ) {
                    Ok(Some(data)) => {
                        dataset_store.data.insert(
                            request_tile_key.clone(),
                            data.map(|path| asset_server.load(translate_tile_server_path(&path))),
                        );
                        continue;
                    }
                    Err(err) => {
                        error!("{err}");
                        dataset_store.failed.insert(request_tile_key.clone());
                        continue;
                    }
                    _ => {}
                };

                if dataset_store.limiter.try_wait().is_err() {
                    break;
                }

                dataset_store.requested.insert(request_tile_key.clone());
                let tile_server = tile_server.clone();
                let dataset = dataset.clone();
                runtime.spawn_background_task(async move |mut task| {
                   let tile_path = tile_server
                       .load_tile(
                           dataset.clone(),
                           &request_projection,
                           &request_tile_key,
                       )
                       .await
                       .inspect_err(|err| error!("{err}"));

                   task.run_on_main_thread(move |ctx| {
                       let _ = ctx
                           .world
                           .run_system_once_with(
                               |(In(tile_key), In(dataset), In(tile_data)): (
                                   In<TileKey>,
                                   In<TileServerDataset>,
                                   In<Result<Option<PathBuf>, TileServerError>>,
                               ),
                                asset_server: Res<AssetServer>,
                                mut store: NonSendMut<TileImageStore>| {
                                   if let Some(dataset_store) = store.dataset_stores.get_mut(&dataset) {
                                   dataset_store.requested.remove(&tile_key);

                                   match tile_data {
                                       Ok(tile_data) => {
                                           dataset_store.data.insert(
                                               tile_key,
                                               tile_data.map(|path| asset_server.load(translate_tile_server_path(&path))),
                                           );
                                       },
                                       Err(err) => {
                                           dataset_store.failed.insert(tile_key);
                                           error!("{err}");
                                       }
                                   };
                                       }
                               },
                               (request_tile_key, dataset, tile_path),
                           )
                           .inspect_err(|err| error!("{err}"));
                   })
                       .await;
               });
            }
        }
    }
}

fn handle_tiling_management(
    mut commands: Commands,
    tiling: Query<(Entity, &mut Tiling, &Tiles, &GeoMapElementOf)>,
    planes: Query<(Entity, &GlobalTransform, &GeoMapPlane)>,
    camera: Query<(&GlobalTransform, &Camera)>,
) {
    if let Ok((camera_transform, camera)) = camera.single() {
        for (tiling_id, mut tiling, tiles, tiling_element_of) in tiling {
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

                    let mut targeted_tiles = HashSet::new();

                    for depth in 0..=target_depth.min(17) {
                        for tile in sub_division.tile_covering(cam_abs_bbox, depth) {
                            targeted_tiles.insert(tile.key.clone());

                            if tiling.spawned_tiles.insert(tile.key.clone()) {
                                commands.entity(tiling_id).with_child((
                                    GeoMapElementOf(plane_id),
                                    Tile {
                                        key: tile.key.clone(),
                                        bb_abs: (tile.bb_min, tile.bb_max),
                                    },
                                    TileOf(tiling_id),
                                    Transform::from_translation(
                                        plane
                                            .abs_to_local(&((tile.bb_min + tile.bb_max) / 2.0))
                                            .extend(1.0 + tile.key.len() as f32 * 0.0001),
                                    ),
                                    Visibility::default(),
                                ));
                            }
                        }
                    }

                    tiling.targeted_tiles = targeted_tiles;
                }
            }
        }
    }
}
fn handle_tile_setup(mut commands: Commands, added_tiles: Query<Entity, Added<Tile>>) {
    for tile_id in added_tiles {
        for (idx, dataset) in [
            TileServerDataset::GibsLayerModisTerraCorrectedReflectanceTrueColor,
            TileServerDataset::SenHubSentinel2L2a,
        ]
        .into_iter()
        .enumerate()
        {
            commands.entity(tile_id).with_child((
                Transform::from_translation(Vec3::new(0.0, 0.0, 1.0 * idx as f32)),
                Visibility::default(),
                TileImage { dataset },
            ));
        }
    }
}
fn handle_tile_target_sync(
    mut commands: Commands,
    tilings: Query<(&Tiling, &Tiles)>,
    tiles: Query<(Entity, &Tile)>,
) {
    for (tiling, tiling_tiles) in tilings {
        for (tile_id, tile) in tiling_tiles
            .0
            .iter()
            .cloned()
            .filter_map(|tile_id| tiles.get(tile_id).ok())
        {
            if tiling.targeted_tiles.contains(&tile.key) {
                commands.entity(tile_id).insert(TileTargeted);
            } else {
                commands.entity(tile_id).remove::<TileTargeted>();
            }
        }
    }
}
