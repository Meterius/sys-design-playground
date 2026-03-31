use bevy::prelude::*;
use bevy_prototype_lyon::geometry::ShapeBuilderBase;
use bevy_prototype_lyon::path::ShapePath;
use bevy_prototype_lyon::plugin::ShapePlugin;
use bevy_prototype_lyon::prelude::ShapeBuilder;
use std::f32::consts::PI;

pub struct GeoMapPlugin {}

impl Plugin for GeoMapPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (geo_map_plane_setup, geo_map_tile_setup));
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

#[derive(Clone)]
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

fn geo_map_plane_setup(
    mut commands: Commands,
    planes: Query<(Entity, &GeoMapPlane), Added<GeoMapPlane>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    asset_server: Res<AssetServer>,
) {
    let dot_mesh = meshes.add(Circle::new(50.0));

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
            .map(|x| LonLatPos(x))
            .map(|x| plane.projection.convert_gcs(x));

        let mut background_border_path = ShapePath::new();
        background_border_path =
            background_border_path.move_to(plane.projection.convert_gcs(LonLatPos::NORTH_WEST));
        for pos in background_border {
            background_border_path = background_border_path.line_to(pos);
        }
        background_border_path = background_border_path.close();

        commands.entity(entity).with_child((
            Transform::from_xyz(0.0, 0.0, -1.0),
            ShapeBuilder::with(&background_border_path)
                .stroke((Color::BLACK, 50.0))
                .fill(Color::srgb(0.4, 0.4, 0.8))
                .build(),
        ));

        for lat in 0..=100 {
            for lon in 0..=100 {
                let lon_lat = LonLatPos(Vec2::new(
                    LonLatPos::MIN_LON
                        + (LonLatPos::MAX_LON - LonLatPos::MIN_LON) * (lon as f32) / 100.0,
                    LonLatPos::MIN_LAT
                        + (LonLatPos::MAX_LAT - LonLatPos::MIN_LAT) * (lat as f32) / 100.0,
                ));
                let pos = plane.projection.convert_gcs(lon_lat.clone());

                commands.entity(entity).with_child((
                    Transform::from_xyz(pos.x, pos.y, 1.0),
                    Mesh2d(dot_mesh.clone()),
                    MeshMaterial2d(materials.add(if (lon_lat.0.y % PI) == 0.0 {
                        Color::srgb(1.0, 0.0, 0.0)
                    } else {
                        Color::BLACK
                    })),
                ));

                commands.entity(entity).with_child((
                    Transform::from_xyz(pos.x, pos.y, 2.0),
                    Text2d(format!(
                        "{:.0} / {:.0}",
                        lon_lat.0.x * 360.0 / (2.0 * PI),
                        lon_lat.0.y * 360.0 / (2.0 * PI)
                    )),
                    TextFont {
                        font: asset_server.load("fonts/calibri-regular.ttf"),
                        font_size: 30.0,
                        ..default()
                    },
                    TextColor(Color::srgb(0.3, 0.3, 1.0)),
                    TextBackgroundColor(Color::NONE),
                ));
            }
        }

        let plane_bottom_left = plane.projection.convert_gcs(LonLatPos::SOUTH_WEST);
        let plane_top_right = plane.projection.convert_gcs(LonLatPos::NORTH_EAST);

        let lon_n = 10;
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
    mut images: Res<Assets<Image>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut asset_server: Res<AssetServer>,
    mut msg_asset: MessageReader<AssetEvent<Image>>,
) {
    let world_image_handle =
        asset_server.load::<Image>("images/equirectangular_world_projection_high_res.png");

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
