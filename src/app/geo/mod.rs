use crate::app::geo::boundaries::load_all_shape_paths;
use crate::geo::sub_division::SubDivision2d;
use bevy::asset::ErasedAssetLoader;
use bevy::math::USizeVec2;
use bevy::prelude::*;
use bevy::tasks::AsyncComputeTaskPool;
use bevy_prototype_lyon::geometry::ShapeBuilderBase;
use bevy_prototype_lyon::path::ShapePath;
use bevy_prototype_lyon::prelude::ShapeBuilder;
use bevy_vector_shapes::painter::ShapePainter;
use bevy_vector_shapes::prelude::*;
use osmpbf::{Element, ElementReader};
use std::f32::consts::PI;
use tokio::sync::mpsc::Receiver;

pub mod boundaries;

pub struct GeoMapPlugin {}

impl Plugin for GeoMapPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                apply_transform,
                geo_map_plane_setup,
                geo_map_tile_setup,
                geo_map_plane_update,
            ),
        );
    }
}

#[derive(Component)]
pub struct GeoMapPlane {
    pub(crate) projection: MercatorProjection,
}

#[derive(Component)]
pub struct GeoMapTile {
    pub south_west: LonLatPos,
    pub north_east: LonLatPos,
}

#[derive(Component)]
pub struct GeoMapDot {
    pub pos: LonLatPos,
    pub label: Option<String>,
}

#[derive(Component)]
#[relationship(relationship_target = GeoMapElements)]
pub struct GeoMapElementOf(pub Entity);

#[derive(Component)]
#[relationship_target(relationship = GeoMapElementOf)]
pub struct GeoMapElements(Vec<Entity>);

#[derive(Component)]
#[require(Transform)]
pub struct GeoMapTransform {
    pub pos: LonLatPos,
}

fn apply_transform(
    transforms: Query<
        (&mut Transform, &GeoMapTransform, &GeoMapElementOf),
        Changed<GeoMapTransform>,
    >,
    planes: Query<&GeoMapPlane>,
) {
    for (mut transform, geo_transform, element_of) in transforms {
        if let Ok(plane) = planes.get(element_of.0) {
            transform.translation = plane
                .projection
                .convert_gcs(geo_transform.pos.clone())
                .extend(transform.translation.z);
        }
    }
}

pub struct GeoMarkerChannel {
    rx: Receiver<(f32, f32, String)>,
}

fn process_markers(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    asset_server: Res<AssetServer>,
    planes: Query<Entity, With<GeoMapPlane>>,
    mut channel: NonSendMut<GeoMarkerChannel>,
) {
    let mut processed = 0;
    while let Some((lon, lat, label)) = channel.rx.blocking_recv()
        && processed < 1
    {
        processed += 1;
        for plane_id in planes {
            let lon_lat = LonLatPos::from_degrees(Vec2::new(lon, lat));
            spawn_marker(
                &mut commands,
                &mut meshes,
                &mut materials,
                &asset_server,
                plane_id,
                lon_lat,
                Some(label.clone()),
                0.01,
            );
        }
    }
}

#[derive(Clone, Debug)]
pub struct LonLatPos(Vec2);

impl LonLatPos {
    const UPPER_LAT: f32 = 0.5 * PI;
    const MAX_LAT: f32 = 0.45 * PI;
    const LOWER_LAT: f32 = -0.5 * PI;
    const MIN_LAT: f32 = -0.45 * PI;

    const MIN_LON: f32 = -PI;
    const LOWER_LON: f32 = -PI;
    const MAX_LON: f32 = PI;
    const UPPER_LON: f32 = PI;

    const NORTH_WEST: Self = Self(Vec2::new(Self::MIN_LON, Self::MAX_LAT));
    const SOUTH_WEST: Self = Self(Vec2::new(Self::MIN_LON, Self::MIN_LAT));
    const NORTH_EAST: Self = Self(Vec2::new(Self::MAX_LON, Self::MAX_LAT));
    const SOUTH_EAST: Self = Self(Vec2::new(Self::MAX_LON, Self::MIN_LAT));

    const LIM_NORTH_WEST: Self = Self(Vec2::new(Self::LOWER_LON, Self::UPPER_LAT));
    const LIM_SOUTH_WEST: Self = Self(Vec2::new(Self::LOWER_LON, Self::LOWER_LAT));
    const LIM_NORTH_EAST: Self = Self(Vec2::new(Self::UPPER_LON, Self::UPPER_LAT));
    const LIM_SOUTH_EAST: Self = Self(Vec2::new(Self::UPPER_LON, Self::LOWER_LAT));

    fn from_degrees(pos: Vec2) -> Self {
        Self(pos * 2.0 * PI / 360.0)
    }

    fn normalized(&self) -> Self {
        Self(Vec2::new(
            (self.0.x - Self::LOWER_LON).rem_euclid(Self::UPPER_LON - Self::LOWER_LON)
                + Self::LOWER_LON,
            (self.0.y - Self::LOWER_LAT).rem_euclid(Self::UPPER_LAT - Self::LOWER_LAT)
                + Self::LOWER_LAT,
        ))
    }
}

pub struct MercatorProjection {
    pub scale: f32,
}

impl MercatorProjection {
    fn convert_gcs(&self, pos: LonLatPos) -> Vec2 {
        Vec2::new(
            self.scale * pos.0.x,
            self.scale * (PI / 4.0 + pos.0.y / 2.0).tan().ln(),
        )
    }

    fn convert_local(&self, pos: Vec2) -> LonLatPos {
        LonLatPos(Vec2::new(
            pos.x / self.scale,
            (pos.y / self.scale).sinh().atan(),
        ))
    }
}

fn spawn_element<'a>(
    commands: &'a mut Commands,
    plane_id: Entity,
    pos: LonLatPos,
    scale: f32,
) -> EntityCommands<'a> {
    commands.spawn((
        GeoMapElementOf(plane_id),
        Transform::from_xyz(0.0, 0.0, 1.0).with_scale(Vec3::ONE * scale),
        Visibility::default(),
        GeoMapTransform { pos },
    ))
}

fn spawn_marker<'a>(
    commands: &'a mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<ColorMaterial>>,
    asset_server: &Res<AssetServer>,
    plane_id: Entity,
    pos: LonLatPos,
    label: Option<String>,
    scale: f32,
) -> EntityCommands<'a> {
    let mut e_commands = spawn_element(commands, plane_id, pos, scale);

    e_commands.with_child((
        Transform::from_xyz(0.0, 0.0, 0.0),
        Mesh2d(meshes.add(Circle::new(10.0))),
        MeshMaterial2d(materials.add(Color::BLACK)),
    ));

    if let Some(label) = label {
        e_commands.with_child((
            Transform::from_xyz(0.0, 30.0, 1.0),
            Text2d(label),
            TextFont {
                font: asset_server.load("fonts/calibri-regular.ttf"),
                font_size: 20.0,
                ..default()
            },
            TextColor(Color::srgb(0.0, 0.0, 0.0)),
            TextBackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.5)),
        ));
    }

    e_commands
}

fn geo_map_startup(world: &mut World) {
    let (tx, rx) = tokio::sync::mpsc::channel(1024);

    world.insert_non_send_resource(GeoMarkerChannel { rx });

    let pool = AsyncComputeTaskPool::get();
    pool.spawn(async move {
        let reader =
            ElementReader::from_path("./assets/datasets/osm/berlin-260329.osm.pbf").unwrap();

        let mut count: usize = 0;
        let mut added_count: usize = 0;
        reader
            .for_each(|el| {
                if count.is_multiple_of(1000000) {
                    info!("Processed {count} elements");
                }

                if let Element::DenseNode(node) = el
                    && let Some((_, name)) = node.tags().find(|(key, _)| *key == "name") {
                        added_count += 1;
                        tx.blocking_send((node.lon() as f32, node.lat() as f32, name.to_owned()))
                            .unwrap();
                        if added_count.is_multiple_of(1000) {
                            info!("Added {added_count} elements")
                        }
                    }
                count += 1;
            })
            .unwrap();

        info!("Processed {count} elements in total!");
    })
    .detach();
}

fn geo_map_plane_update(
    mut painter: ShapePainter,
    planes: Query<(&GlobalTransform, &GeoMapPlane)>,
    camera: Query<(&GlobalTransform, &Camera)>,
) {
    if let Ok((camera_transform, camera)) = camera.single() {
        for (plane_transform, plane) in planes {
            let plane_bottom_left = plane.projection.convert_gcs(LonLatPos::SOUTH_WEST);
            let plane_top_right = plane.projection.convert_gcs(LonLatPos::NORTH_EAST);

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

                let target_depth = sub_division.min_depth_for_tile_count(cam_area, USizeVec2::new(10, 10));

                for offset in [-1, 0, 1].into_iter() {
                    let depth = target_depth as isize + offset;

                    if depth >= 1 {
                        for tile in sub_division.tile_covering(cam_area, depth as usize) {
                            painter.set_translation((tile.bb_min + tile.bb_max).extend(0.0) / 2.0);
                            painter.hollow = true;
                            painter.color = match offset {
                                -1 => Color::srgba(1.0, 0.0, 1.0, 0.5),
                                1 => Color::srgba(1.0, 1.0, 0.0, 0.5),
                                _ => Color::srgba(0.0, 1.0, 0.0, 0.8),
                            };
                            painter.thickness = tile.bb_size.max_element() * 0.01;
                            painter.rect(tile.bb_size);
                        }
                    }
                }
            }
        }
    }
}

fn geo_map_plane_setup(
    mut commands: Commands,
    planes: Query<(Entity, &GeoMapPlane), Added<GeoMapPlane>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    asset_server: Res<AssetServer>,
) {
    for (entity, plane) in planes {
        let background_border = (0..=10)
            .map(|idx| {
                LonLatPos::NORTH_WEST.0
                    + (idx as f32 / 10.0) * (LonLatPos::NORTH_EAST.0 - LonLatPos::NORTH_WEST.0)
            })
            .chain((0..=10).map(|idx| {
                LonLatPos::NORTH_EAST.0
                    + (idx as f32 / 10.0) * (LonLatPos::SOUTH_EAST.0 - LonLatPos::NORTH_EAST.0)
            }))
            .chain((0..=10).map(|idx| {
                LonLatPos::SOUTH_EAST.0
                    + (idx as f32 / 10.0) * (LonLatPos::SOUTH_WEST.0 - LonLatPos::SOUTH_EAST.0)
            }))
            .chain((0..=10).map(|idx| {
                LonLatPos::SOUTH_WEST.0
                    + (idx as f32 / 10.0) * (LonLatPos::NORTH_WEST.0 - LonLatPos::SOUTH_WEST.0)
            }))
            .map(LonLatPos)
            .map(|x| plane.projection.convert_gcs(x));

        let mut background_border_path = ShapePath::new();
        background_border_path =
            background_border_path.move_to(plane.projection.convert_gcs(LonLatPos::NORTH_WEST));
        for pos in background_border {
            background_border_path = background_border_path.line_to(pos);
        }
        background_border_path = background_border_path.close();

        commands.entity(entity).with_child((
            Transform::from_xyz(0.0, 0.0, -2.0),
            ShapeBuilder::with(&background_border_path)
                .stroke((Color::BLACK, 50.0))
                .fill(Color::srgb(0.4, 0.4, 0.8))
                .build(),
        ));

        for path in load_all_shape_paths("assets/datasets/geojson", |pos| {
            plane.projection.convert_gcs(LonLatPos::from_degrees(pos))
        }) {
            commands.entity(entity).with_child((
                Transform::from_xyz(0.0, 0.0, 2.0),
                ShapeBuilder::with(&path)
                    .stroke((Color::srgba(1.0, 0.0, 0.0, 0.3), 5.0))
                    .build(),
            ));
        }

        let lat_n = 20;
        let lon_n = 20;
        for lat in 0..=lat_n {
            for lon in 0..=lon_n {
                let lon_lat = LonLatPos(Vec2::new(
                    LonLatPos::MIN_LON
                        + (LonLatPos::MAX_LON - LonLatPos::MIN_LON) * (lon as f32) / lon_n as f32,
                    LonLatPos::MIN_LAT
                        + (LonLatPos::MAX_LAT - LonLatPos::MIN_LAT) * (lat as f32) / lat_n as f32,
                ));

                spawn_marker(
                    &mut commands,
                    &mut meshes,
                    &mut materials,
                    &asset_server,
                    entity,
                    lon_lat.clone(),
                    Some(format!(
                        "{:.0} / {:.0}",
                        lon_lat.0.x * 360.0 / (2.0 * PI),
                        lon_lat.0.y * 360.0 / (2.0 * PI)
                    )),
                    1.0,
                );
            }
        }

        let plane_bottom_left = plane.projection.convert_gcs(LonLatPos::SOUTH_WEST);
        let plane_top_right = plane.projection.convert_gcs(LonLatPos::NORTH_EAST);

        let lon_n = 100;
        let lat_n = ((lon_n as f32) * (plane_top_right.y - plane_bottom_left.y)
            / (plane_top_right.x - plane_bottom_left.x))
            .ceil() as usize;

        for lat in 0..lat_n {
            for lon in 0..lon_n {
                let bottom_left = plane_bottom_left
                    + (plane_top_right - plane_bottom_left)
                        * Vec2::new(lon as f32 / lon_n as f32, lat as f32 / lat_n as f32);
                let top_right = plane_bottom_left
                    + (plane_top_right - plane_bottom_left)
                        * Vec2::new(
                            (lon as f32 + 1.0) / lon_n as f32,
                            (lat as f32 + 1.0) / lat_n as f32,
                        );

                commands.entity(entity).with_child(GeoMapTile {
                    south_west: plane.projection.convert_local(bottom_left),
                    north_east: plane.projection.convert_local(top_right),
                });
            }
        }
    }
}

fn geo_map_tile_setup(
    mut commands: Commands,
    added_tiles: Query<(Entity, &ChildOf, &GeoMapTile), Added<GeoMapTile>>,
    all_tiles: Query<(Entity, &ChildOf, &GeoMapTile)>,
    planes: Query<&GeoMapPlane>,
    images: Res<Assets<Image>>,
    asset_server: Res<AssetServer>,
    mut msg_asset: MessageReader<AssetEvent<Image>>,
) {
    let world_image_handle =
        asset_server.load::<Image>("images/equirectangular_world_projection.png");

    let world_image_changed = msg_asset.read().any(|msg| match msg {
        AssetEvent::Added { id } | AssetEvent::Modified { id } => id == &world_image_handle.id(),
        _ => false,
    });

    let tiles = if world_image_changed {
        all_tiles.iter().collect::<Vec<_>>()
    } else {
        added_tiles.iter().collect::<Vec<_>>()
    };

    let world_image_size = if let Some(world_image) = images.get(world_image_handle.id()) {
        world_image.size_f32()
    } else {
        Vec2::ONE
    };

    for (entity, child_of, tile) in tiles.into_iter() {
        if let Ok(plane) = planes.get(child_of.parent()) {
            let margin = Vec2::new(0.0, 0.0);
            let top_right = plane.projection.convert_gcs(tile.north_east.clone()) - margin / 2.0;
            let bottom_left = plane.projection.convert_gcs(tile.south_west.clone()) + margin / 2.0;

            let size = top_right - bottom_left;
            let center = (bottom_left + top_right) / 2.0;

            commands
                .entity(entity)
                .insert(Transform::from_translation(center.extend(0.0)))
                .insert(Sprite {
                    image: world_image_handle.clone(),
                    custom_size: Some(size),
                    rect: Some(Rect::from_corners(
                        world_image_size * (tile.south_west.0 - LonLatPos::LIM_NORTH_WEST.0)
                            / (LonLatPos::LIM_SOUTH_EAST.0 - LonLatPos::LIM_NORTH_WEST.0),
                        world_image_size * (tile.north_east.0 - LonLatPos::LIM_NORTH_WEST.0)
                            / (LonLatPos::LIM_SOUTH_EAST.0 - LonLatPos::LIM_NORTH_WEST.0),
                    )),
                    ..default()
                });
        }
    }
}
