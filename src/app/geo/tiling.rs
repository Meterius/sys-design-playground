use crate::app::geo::{GeoMapElementOf, GeoMapPlane, GeoMapPlaneView};
use crate::app::settings::Settings;
use crate::geo::coords::Projection2D;
use crate::geo::sub_division::{SubDivision2d, TileKey};
use crate::geo::tiling::{TileServer, TileServerDataset, TileServerError};
use bevy::ecs::system::RunSystemOnce;
use bevy::math::USizeVec2;
use bevy::math::bounding::Aabb2d;
use bevy::prelude::*;
use bevy_tokio_tasks::TokioTasksRuntime;
use bevy_vector_shapes::painter::ShapePainter;
use bevy_vector_shapes::prelude::RectPainter;
use bevy_vector_shapes::shapes::ThicknessType;
use itertools::Itertools;
use priority_queue::PriorityQueue;
use ratelimit::Ratelimiter;
use std::cmp::Reverse;
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
                (
                    handle_tiling_management,
                    handle_tile_setup,
                    handle_tile_target_sync,
                    handle_tile_image_loading,
                    handle_loaded_tile_image,
                    handle_tile_image_fade,
                )
                    .chain(),
                (handle_tile_debug, handle_tile_image_debug).chain(),
            ),
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

#[derive(Component)]
#[require(Tiles)]
pub struct Tiling {
    pub target_depth: usize,

    max_depth: usize,

    target_depth_fac: f32,
    target_count: usize,

    spawned_tiles: PriorityQueue<TileKey, Reverse<usize>>,
    spawned_tiles_untargeted_capacity: usize,

    targeted_tiles: HashSet<TileKey>,
}

impl Tiling {
    const MAX_DESPAWN_TILE_PER_TICK: usize = 128;

    pub fn new(target_count: usize) -> Self {
        Self {
            max_depth: 17,
            target_count,
            target_depth_fac: 0.0,
            target_depth: 0,
            spawned_tiles: PriorityQueue::new(),
            spawned_tiles_untargeted_capacity: 1024,
            targeted_tiles: HashSet::new(),
        }
    }
}

#[derive(Default, Component)]
#[relationship_target(relationship = TileOf)]
pub struct Tiles(Vec<Entity>);

#[derive(Component)]
#[relationship(relationship_target = Tiles)]
pub struct TileOf(Entity);

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

fn handle_tile_image_debug(
    settings: Res<Settings>,
    tile_images: Query<(&GlobalTransform, &Sprite), With<TileImage>>,
    mut painter: ShapePainter,
) {
    if settings.debug_mode {
        for (tile_image_transform, tile_image_sprite) in tile_images.iter() {
            if let Some(size) = tile_image_sprite.custom_size.clone() {
                painter.set_translation(tile_image_transform.translation());
                painter.translate(Vec3::new(0.0, 0.0, 200.0));
                painter.color = Color::srgb(0.0, 1.0, 0.0);
                painter.hollow = true;
                painter.thickness = 0.001 * size.max_element();
                painter.thickness_type = ThicknessType::World;
                painter.rect(size);
            }
        }
    }
}

fn handle_tile_debug(
    settings: Res<Settings>,
    tiles: Query<(&GlobalTransform, &Tile, &GeoMapElementOf)>,
    planes: Query<&GeoMapPlane>,
    mut painter: ShapePainter,
) {
    if settings.debug_mode {
        for (tile_transform, tile, tile_element_of) in tiles.iter() {
            if let Some(plane) = planes.get(tile_element_of.0).ok() {
                let size = (tile.bb_abs.1 - tile.bb_abs.0) * plane.scale;
                painter.set_translation(tile_transform.translation());
                painter.translate(Vec3::new(0.0, 0.0, 100.0));
                painter.color = Color::srgb(1.0, 0.0, 0.0);
                painter.hollow = true;
                painter.thickness = 0.001 * size.max_element();
                painter.thickness_type = ThicknessType::World;
                painter.rect(size);
            }
        }
    }
}

fn handle_tile_image_fade(
    mut tile_image: Query<(&ChildOf, &mut Sprite)>,
    tiles: Query<(&Tile, &TileOf)>,
    tiling: Query<&Tiling>,
) {
    for (tile_image_child_of, mut tile_image_sprite) in tile_image.iter_mut() {
        if let Some((tile, tile_of)) = tiles.get(tile_image_child_of.0).ok()
            && let Some(tiling) = tiling.get(tile_of.0).ok()
        {
            let fac = (tiling.target_depth as f32 - tile.key.len() as f32
                + tiling.target_depth_fac)
                .clamp(0.0, 0.75)
                / 0.75;

            tile_image_sprite.color.set_alpha(fac);
        }
    }
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
    mut tick: Local<usize>,
    mut commands: Commands,
    tiles: Query<&Tile>,
    tiling: Query<(Entity, &mut Tiling, &Tiles, &GeoMapElementOf)>,
    planes: Query<(Entity, &GeoMapPlane, &GeoMapPlaneView)>,
) {
    for (tiling_id, mut tiling, tiling_tiles, tiling_element_of) in tiling {
        if let Ok((plane_id, plane, plane_view)) = planes.get(tiling_element_of.0)
            && let Some(view_gcs_bbox) = plane_view.view_gcs.as_ref()
        {
            let view_abs_bbox = (
                plane.projection.gcs_to_abs(&view_gcs_bbox.0),
                plane.projection.gcs_to_abs(&view_gcs_bbox.1),
            );

            let sub_division = SubDivision2d {
                area: Aabb2d::new(
                    plane.projection.abs_pos(),
                    plane.projection.abs_size() / 2.0,
                ),
            };

            let view_abs_size = view_abs_bbox.1 - view_abs_bbox.0;

            let target_depth = sub_division.min_depth_for_tile_count(
                view_abs_size,
                USizeVec2::new(tiling.target_count, tiling.target_count),
            );

            let prev_target_depth_area_size = sub_division.area_size_for_min_depth_for_tile_count(
                if target_depth == 0 {
                    0
                } else {
                    target_depth - 1
                },
                USizeVec2::new(tiling.target_count, tiling.target_count),
            );

            let target_depth_area_size = sub_division.area_size_for_min_depth_for_tile_count(
                target_depth,
                USizeVec2::new(tiling.target_count, tiling.target_count),
            );

            tiling.target_depth = target_depth;
            tiling.target_depth_fac = ((view_abs_size - prev_target_depth_area_size)
                / (target_depth_area_size - prev_target_depth_area_size))
                .max_element()
                .clamp(0.0, 1.0);

            let mut targeted_tiles = HashSet::new();

            for depth in 0..=(target_depth + 1).min(tiling.max_depth) {
                for tile in sub_division.tile_covering(view_abs_bbox, depth) {
                    targeted_tiles.insert(tile.key.clone());

                    if tiling
                        .spawned_tiles
                        .push(tile.key.clone(), Reverse(*tick))
                        .is_none()
                    {
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

            let mut to_remove_tiles = HashSet::new();
            while tiling.spawned_tiles.len() - targeted_tiles.len()
                >= tiling.spawned_tiles_untargeted_capacity
                && to_remove_tiles.len() < Tiling::MAX_DESPAWN_TILE_PER_TICK
                && let Some((tile_key, _)) = tiling.spawned_tiles.pop()
            {
                to_remove_tiles.insert(tile_key);
            }

            if !to_remove_tiles.is_empty() {
                for &tile_id in tiling_tiles.0.iter() {
                    if let Some(tile) = tiles.get(tile_id).ok()
                        && to_remove_tiles.remove(&tile.key)
                    {
                        commands.entity(tile_id).despawn();
                    }
                }

                if !to_remove_tiles.is_empty() {
                    warn!(
                        "Could find tiles corresponding to despawned tiles: {}",
                        to_remove_tiles
                            .into_iter()
                            .map(|key| format!("{:?}", key))
                            .join(", ")
                    );
                }
            }

            tiling.targeted_tiles = targeted_tiles;
        }
    }

    *tick += 1;
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
