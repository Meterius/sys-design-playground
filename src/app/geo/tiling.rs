use crate::app::geo::{GeoMapElementOf, GeoMapPlane};
use crate::geo::coords::{BoundedMercatorProjection, RadLonLatVec2};
use crate::geo::sub_division::{SubDivision2d, TileKey};
use crate::geo::tiling::TileServer;
use bevy::math::USizeVec2;
use bevy::prelude::*;
use bevy_tokio_tasks::TokioTasksRuntime;
use std::collections::{HashMap, HashSet};
use std::f32::consts::PI;
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
                geo_map_plane_tiling_update,
                geo_map_plane_tile_image_store_update,
                geo_map_plane_tiling_update_sprite,
            )
                .chain(),
        );
    }
}

fn startup(world: &mut World) {
    let runtime = world.get_resource_mut::<TokioTasksRuntime>().unwrap();

    let (tile_data_sender, tile_data_receiver) = async_channel::unbounded();
    let (tile_request_sender, tile_request_receiver) =
        async_channel::unbounded::<(TileKey, (RadLonLatVec2, RadLonLatVec2))>();

    let tile_server = TileServer::new(PathBuf::from_iter(["assets", "cache"]));

    for _ in 0..32 {
        let tile_request_receiver = tile_request_receiver.clone();
        let tile_data_sender = tile_data_sender.clone();
        let tile_server = tile_server.clone();
        let projection = BoundedMercatorProjection {
            lat_min: -0.45 * PI,
            lat_max: 0.45 * PI,
            scale: 5000.0,
        };

        runtime.spawn_background_task(|_| async move {
            while let Ok((tile_key, _)) = tile_request_receiver.recv().await {
                let tile_res = tile_server
                    .load_tile(&projection, &tile_key)
                    .await
                    .inspect_err(|err| error!("{err}"));

                if let Ok(tile_img_path) = tile_res {
                    let rel_tile_img_path = PathBuf::from_iter(tile_img_path.iter().skip(1));
                    let _ = tile_data_sender
                        .send((tile_key, rel_tile_img_path))
                        .await
                        .inspect_err(|err| error!("{err:?}"));
                }
            }
        });
    }

    world.insert_non_send_resource(GeoMapTileImageStore {
        tiles: HashMap::new(),
        requested: HashSet::new(),
        tile_data_receiver,
        tile_request_sender,
    });
}

pub struct GeoMapTileImageStore {
    requested: HashSet<TileKey>,
    tiles: HashMap<TileKey, Handle<Image>>,
    tile_data_receiver: async_channel::Receiver<(TileKey, PathBuf)>,
    tile_request_sender: async_channel::Sender<(TileKey, (RadLonLatVec2, RadLonLatVec2))>,
}

fn geo_map_plane_tile_image_store_update(
    mut store: NonSendMut<GeoMapTileImageStore>,
    asset_server: Res<AssetServer>,
) {
    let mut max_recv = 10;
    while max_recv >= 1
        && let Ok((key, image_key)) = store.tile_data_receiver.try_recv()
    {
        store.tiles.insert(key, asset_server.load(image_key));
        max_recv -= 1;
    }
}

fn geo_map_plane_tiling_update_sprite(
    mut commands: Commands,
    mut store: NonSendMut<GeoMapTileImageStore>,
    tiles_without_sprite: Query<(Entity, &GeoMapPlaneTile), Without<Sprite>>,
) {
    for (tile_id, tile) in tiles_without_sprite {
        if let Some(handle) = store.tiles.get(&tile.key) {
            commands.entity(tile_id).insert(Sprite {
                image: handle.clone(),
                custom_size: Some(tile.bb_abs.1 - tile.bb_abs.0),
                ..default()
            });
        } else if !store.requested.contains(&tile.key) {
            store.requested.insert(tile.key.clone());

            let _ = store
                .tile_request_sender
                .try_send((tile.key.clone(), tile.bb_gcs.clone()))
                .inspect_err(|err| error!("{err:?}"));
        }
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
}

fn geo_map_plane_tiling_update(
    mut commands: Commands,
    added_tiles: Query<(Entity, &GeoMapPlaneTile), Added<GeoMapPlaneTile>>,
) {
    for (tile_id, tile) in added_tiles {
        commands.entity(tile_id).insert((
            Transform::from_translation(
                ((tile.bb_abs.0 + tile.bb_abs.1) / 2.0)
                    .extend(1.0 + tile.key.len() as f32 * 0.0001),
            ),
            Visibility::default(),
        ));
    }
}

fn geo_map_plane_tiling_tile_management(
    mut commands: Commands,
    tiling: Query<(&mut GeoMapPlaneTiling, &GeoMapElementOf)>,
    planes: Query<(Entity, &GlobalTransform, &GeoMapPlane)>,
    camera: Query<(&GlobalTransform, &Camera)>,
) {
    if let Ok((camera_transform, camera)) = camera.single() {
        for (mut tiling, tiling_element_of) in tiling {
            if let Ok((plane_id, _, plane)) = planes.get(tiling_element_of.0) {
                let plane_size = plane.projection.abs_size();
                let plane_pos = plane.projection.abs_pos();

                let plane_bottom_left = plane_pos - plane_size / 2.0;
                let plane_top_right = plane_pos + plane_size / 2.0;

                let cam_bottom_left = camera
                    .ndc_to_world(camera_transform, Vec2::NEG_ONE.extend(0.0))
                    .map(Vec3::xy);
                let cam_top_right = camera
                    .ndc_to_world(camera_transform, Vec2::ONE.extend(0.0))
                    .map(Vec3::xy);

                if let Some(cam_bottom_left) = cam_bottom_left
                    && let Some(cam_top_right) = cam_top_right
                {
                    let cam_area = (cam_bottom_left, cam_top_right);
                    let sub_division =
                        SubDivision2d::from_corners(plane_bottom_left, plane_top_right);

                    let target_depth = sub_division.min_depth_for_tile_count(
                        cam_area,
                        USizeVec2::new(tiling.target_count, tiling.target_count),
                    );

                    for depth in 0..=target_depth.min(9) {
                        for tile in sub_division.tile_covering(cam_area, depth) {
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
                                        },
                                    ))
                                    .id();

                                e.insert(tile_id);
                            }
                        }
                    }
                }
            }
        }
    }
}
