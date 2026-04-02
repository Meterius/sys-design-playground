use std::collections::{HashMap, HashSet};
use std::fs;
use bevy::math::USizeVec2;
use bevy::prelude::*;
use bevy::tasks::AsyncComputeTaskPool;
use bevy_prototype_lyon::prelude::{ShapeBuilder, ShapeBuilderBase};
use crate::app::geo::{GeoMapElementOf, GeoMapPlane, GeoMapTransform};
use crate::geo::coords::{LonLatVec2, RadLonLatVec2};
use crate::geo::sub_division::{SubDivision2d, SubDivisionKey};
use bevy_prototype_lyon::shapes;
use bevy_tokio_tasks::TokioTasksRuntime;
use futures::task::waker;
use itertools::Itertools;
use crate::geo::gibs::{fetch_epsg4326_image, GibsEpsg4326Params, LAYER_MODIS_TERRA_CORRECTED_REFLECTANCE_TRUE_COLOR};

#[derive(Default)]
pub struct GeoMapTilingPlugin { }

impl Plugin for GeoMapTilingPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, startup);
        app.add_systems(Update, (geo_map_plane_tiling_tile_management, geo_map_plane_tiling_update, geo_map_plane_tile_image_store_update, geo_map_plane_tiling_update_sprite).chain());
    }
}

fn startup(
    world: &mut World,
) {
    let runtime = world.get_resource_mut::<TokioTasksRuntime>().unwrap();

    let (tile_data_sender, tile_data_receiver) = async_channel::unbounded();
    let (tile_request_sender, mut tile_request_receiver) = async_channel::unbounded::<(Vec<SubDivisionKey>, (RadLonLatVec2, RadLonLatVec2))>();

    for _ in 0..32 {
        let tile_request_receiver = tile_request_receiver.clone();
        let tile_data_sender = tile_data_sender.clone();

        runtime.spawn_background_task(|_| async move {
            let client = reqwest::Client::new();

            while let Ok((path, bb_gcs)) = tile_request_receiver.recv().await {
                let layers = LAYER_MODIS_TERRA_CORRECTED_REFLECTANCE_TRUE_COLOR.to_owned();
                let path_key = path.iter().map(|key| match key {
                    SubDivisionKey::BottomLeft => "BL",
                    SubDivisionKey::BottomRight => "BR",
                    SubDivisionKey::TopLeft => "TL",
                    SubDivisionKey::TopRight => "TR",
                }).join("_");

                let cache_key = format!("cache/gibs/{layers}_{path_key}.png");
                let asset_key = format!("assets/{cache_key}");

                let bb_gcs_min_deg = LonLatVec2::from(bb_gcs.0.clone());
                let bb_gcs_max_deg = LonLatVec2::from(bb_gcs.1.clone());

                info!("{path:?} {bb_gcs_min_deg:?} {bb_gcs_max_deg:?}");

                if !fs::exists(&asset_key).inspect_err(|err| error!("Looking up cache key error: {err:?}")).unwrap_or(false) {
                    info!("Fetching {cache_key}");

                    if let Ok(data) = fetch_epsg4326_image(&client, GibsEpsg4326Params {
                        layers: layers.clone(),
                        bbox: (bb_gcs_min_deg.x, bb_gcs_min_deg.y, bb_gcs_max_deg.x, bb_gcs_max_deg.y),
                        size: (256, 256),
                    })
                        .await
                        .inspect_err(|err| error!("Fetching image error: {err:?}")) {
                        data.save(&asset_key).unwrap();
                        info!("Fetched {cache_key}");
                    }
                } else {
                    info!("Found cached {cache_key}");
                }

                let _ = tile_data_sender.send((path, cache_key)).await.inspect_err(|err| error!("{err:?}"));
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
    requested: HashSet<Vec<SubDivisionKey>>,
    tiles: HashMap<Vec<SubDivisionKey>, Handle<Image>>,
    tile_data_receiver: async_channel::Receiver<(Vec<SubDivisionKey>, String)>,
    tile_request_sender: async_channel::Sender<(Vec<SubDivisionKey>, (RadLonLatVec2, RadLonLatVec2))>,
}

fn geo_map_plane_tile_image_store_update(
    mut store: NonSendMut<GeoMapTileImageStore>,
    asset_server: Res<AssetServer>,
) {
    let mut max_recv = 10;
    while max_recv >= 1 && let Ok((key, image_key)) = store.tile_data_receiver.try_recv() {
        store.tiles.insert(key, asset_server.load(&image_key));
        max_recv -= 1;
    }
}

fn geo_map_plane_tiling_update_sprite(
    mut commands: Commands,
    mut store: NonSendMut<GeoMapTileImageStore>,
    tiles_without_sprite: Query<(Entity, &GeoMapPlaneTile), Without<Sprite>>,
    asset_server: Res<AssetServer>,
) {
    for (tile_id, tile) in tiles_without_sprite {
        if let Some(handle) = store.tiles.get(&tile.path) {
            commands.entity(tile_id).insert(Sprite {
                image: handle.clone(),
                custom_size: Some(tile.bb_abs.1 - tile.bb_abs.0),
                ..default()
            });
        } else if !store.requested.contains(&tile.path) {
            store.requested.insert(tile.path.clone());

            let path_key = tile.path.iter().map(|key| match key {
                SubDivisionKey::BottomLeft => "BL",
                SubDivisionKey::BottomRight => "BR",
                SubDivisionKey::TopLeft => "TL",
                SubDivisionKey::TopRight => "TR",
            }).join("_");

            let layers = LAYER_MODIS_TERRA_CORRECTED_REFLECTANCE_TRUE_COLOR.to_owned();
            let cache_key = format!("cache/gibs/{layers}_{path_key}.png");
            let asset_key = format!("assets/{cache_key}");

            if fs::exists(&asset_key).inspect_err(|err| error!("Looking up cache key error: {err:?}")).unwrap_or(false) {
                store.tiles.insert(tile.path.clone(), asset_server.load(&cache_key));
            } else {
                let _ = store.tile_request_sender.try_send((tile.path.clone(), tile.bb_gcs.clone()))
                    .inspect_err(|err| error!("{err:?}"));
            }

        }
    }
}

#[derive(Component)]
pub struct GeoMapPlaneTiling {
    pub target_count: usize,
    tile_map: HashMap<Vec<SubDivisionKey>, Entity>,
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
    pub path: Vec<SubDivisionKey>,
    pub bb_gcs: (RadLonLatVec2, RadLonLatVec2),
    pub bb_abs: (Vec2, Vec2),
}

fn geo_map_plane_tiling_update(
    mut commands: Commands,
    added_tiles: Query<(Entity, &GeoMapPlaneTile), Added<GeoMapPlaneTile>>,
) {
    for (tile_id, tile) in added_tiles {
        let shape = shapes::Rectangle {
            origin: shapes::RectangleOrigin::Center,
            extents: tile.bb_abs.1 - tile.bb_abs.0,
            radii: None,
        };

        commands.entity(tile_id).insert((
            Transform::from_translation(((tile.bb_abs.0 + tile.bb_abs.1) / 2.0).extend(1.0 + tile.path.len() as f32 * 0.0001)),
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
            if let Ok((plane_id, plane_transform, plane)) = planes.get(tiling_element_of.0) {
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
                    let sub_division = SubDivision2d::from_corners(plane_bottom_left, plane_top_right);

                    let target_depth = sub_division.min_depth_for_tile_count(cam_area, USizeVec2::new(tiling.target_count, tiling.target_count));

                    for depth in 0..=target_depth.min(9) {
                        for tile in sub_division.tile_covering(cam_area, depth) {
                            if !tiling.tile_map.contains_key(&tile.path) {
                                let bb_gcs = (plane.projection.abs_to_gcs(&tile.bb_min), plane.projection.abs_to_gcs(&tile.bb_max));

                                let tile_id = commands.entity(plane_id).with_child((
                                    GeoMapElementOf(plane_id),
                                    GeoMapPlaneTile { path: tile.path.clone(), bb_abs: (tile.bb_min, tile.bb_max), bb_gcs }
                                )).id();

                                tiling.tile_map.insert(tile.path, tile_id);
                            }
                        }
                    }
                }
            }
        }
    }
}