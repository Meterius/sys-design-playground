use crate::app::geo::map::Map;
use crate::app::geo::tiling::setup_tiles;
use crate::app::utils::SoftExpect;
use crate::geo::sub_division::TileKey;
use crate::geo::tiling::{TileServer, TileServerDataset, TileServerError};
use bevy::ecs::system::RunSystemOnce;
use bevy::prelude::*;
use bevy_tokio_tasks::TokioTasksRuntime;
use ratelimit::Ratelimiter;
use std::cmp::Reverse;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

#[derive(Default)]
pub struct TileFetcherPlugin {}

impl Plugin for TileFetcherPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, startup);
        app.add_systems(
            Update,
            (handle_tile_image_loading, handle_tile_image_sprite_loaded)
                .chain()
                .after(setup_tiles),
        );
    }
}

fn startup(world: &mut World) {
    world.insert_non_send_resource(TileServer::new(
        256,
        PathBuf::from_iter(["assets", "cache"]),
    ));

    world.insert_non_send_resource(TileImageStore {
        dataset_stores: HashMap::from_iter([
            (
                TileServerDataset::SenHubSentinel2L2a,
                TileImageDatasetStore::new(Ratelimiter::new(4)),
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

#[derive(Component, Reflect, Eq, PartialEq, Hash)]
pub struct TileImageRequest {
    pub key: TileKey,
    pub dataset: TileServerDataset,
    pub priority: isize,
}

#[derive(Component, Reflect)]
pub struct TileImageResponse {
    pub key: TileKey,
    pub data: Option<Handle<Image>>,
}

#[derive(Component)]
#[relationship(relationship_target = MapWithRequests)]
pub struct TileImageRequestWithMap(pub Entity);

#[derive(Component)]
#[relationship_target(relationship = TileImageRequestWithMap)]
pub struct MapWithRequests(Vec<Entity>);

fn handle_tile_image_loading(
    mut commands: Commands,
    mut store: NonSendMut<TileImageStore>,
    tile_server: NonSend<TileServer>,
    runtime: Res<TokioTasksRuntime>,
    requests: Query<(Entity, &TileImageRequest, &TileImageRequestWithMap)>,
    maps: Query<&Map>,
    asset_server: Res<AssetServer>,
) {
    let mut requests_by_dataset = HashMap::new();

    for (request_id, request, &TileImageRequestWithMap(map_id)) in requests {
        if let Some(map) = maps.get(map_id).ok().soft_expect("")
            && let Some(dataset_store) = store.dataset_stores.get(&request.dataset).soft_expect("")
        {
            if let Some(handle) = dataset_store.data.get(&request.key) {
                commands.entity(request_id).remove::<TileImageRequest>();
                commands.entity(request_id).insert(TileImageResponse {
                    key: request.key.clone(),
                    data: handle.clone(),
                });
            } else if dataset_store.failed.contains(&request.key) {
                commands.entity(request_id).remove::<TileImageRequest>();
                commands.entity(request_id).insert(TileImageResponse {
                    key: request.key.clone(),
                    data: None,
                });
            } else if !dataset_store.requested.contains(&request.key) {
                let requests = requests_by_dataset
                    .entry(request.dataset.clone())
                    .or_insert_with(Vec::new);
                requests.push((request.key.clone(), map.projection, request.priority));
            }
        }
    }

    fn translate_tile_server_path(path: &Path) -> PathBuf {
        PathBuf::from_iter(path.iter().skip(1))
    }

    for (dataset, mut requests) in requests_by_dataset.into_iter() {
        if let Some(dataset_store) = store.dataset_stores.get_mut(&dataset) {
            requests.sort_by_key(|(_, _, priority)| Reverse(*priority));

            for (request_tile_key, request_projection, _) in requests.into_iter() {
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
                        .inspect_err(|err| {
                            match err {
                                TileServerError::RetryError => {},
                                err => error!("{err}"),
                            };
                        });


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
                                            Err(_) => {
                                                dataset_store.failed.insert(tile_key);
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

#[derive(Component)]
pub struct TileImageSprite {
    pub size: Option<Vec2>,
}

pub fn handle_tile_image_sprite_loaded(
    mut commands: Commands,
    tiles: Query<(Entity, &TileImageSprite, &TileImageResponse), Added<TileImageResponse>>,
) {
    for (tile_id, tile_sprite, tile_res) in tiles {
        if let Some(image) = tile_res.data.clone() {
            commands.entity(tile_id).insert(Sprite {
                image,
                custom_size: tile_sprite.size,
                ..default()
            });
        }
    }
}
