use crate::app::geo::boundaries::load_all_shape_paths;
use crate::geo::coords::{BoundedMercatorProjection, LonLatVec2, RadLonLatVec2};
use bevy::asset::ErasedAssetLoader;
use bevy::prelude::*;
use bevy_prototype_lyon::geometry::ShapeBuilderBase;
use bevy_prototype_lyon::path::ShapePath;
use bevy_prototype_lyon::prelude::ShapeBuilder;
use crate::app::geo::tiling::GeoMapTilingPlugin;

pub mod boundaries;
pub mod tiling;

pub struct GeoMapPlugin {}

impl Plugin for GeoMapPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (apply_transform, geo_map_plane_setup));
        app.add_plugins(GeoMapTilingPlugin::default());
    }
}

#[derive(Component)]
pub struct GeoMapPlane {
    pub(crate) projection: BoundedMercatorProjection,
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
    pub pos: RadLonLatVec2,
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
                .gcs_to_abs(&geo_transform.pos)
                .extend(transform.translation.z);
        }
    }
}

fn spawn_element<'a>(
    commands: &'a mut Commands,
    plane_id: Entity,
    pos: RadLonLatVec2,
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
    pos: RadLonLatVec2,
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

fn geo_map_plane_setup(
    mut commands: Commands,
    planes: Query<(Entity, &GeoMapPlane), Added<GeoMapPlane>>,
    meshes: ResMut<Assets<Mesh>>,
    materials: ResMut<Assets<ColorMaterial>>,
    asset_server: Res<AssetServer>,
) {
    for (entity, plane) in planes {
        let plane_pos = plane.projection.abs_pos();
        let plane_size = plane.projection.abs_size();

        let background_border = [
            plane_pos + Vec2::new(-1.0, -1.0) * plane_size / 2.0,
            plane_pos + Vec2::new(-1.0, 1.0) * plane_size / 2.0,
            plane_pos + Vec2::new(1.0, 1.0) * plane_size / 2.0,
            plane_pos + Vec2::new(1.0, -1.0) * plane_size / 2.0,
            plane_pos + Vec2::new(-1.0, -1.0) * plane_size / 2.0,
        ];

        let mut background_border_path = ShapePath::new();
        background_border_path = background_border_path.move_to(background_border[0]);
        for pos in background_border {
            background_border_path = background_border_path.line_to(pos);
        }
        background_border_path = background_border_path.close();

        commands.entity(entity).with_child((
            Transform::from_xyz(0.0, 0.0, -2.0),
            ShapeBuilder::with(&background_border_path)
                .stroke((Color::BLACK, 50.0))
                .fill(Color::srgba(0.4, 0.4, 0.8, 0.25))
                .build(),
        ));

        for path in load_all_shape_paths("assets/datasets/geojson", |pos| {
            plane.projection.gcs_to_abs(&LonLatVec2::from(pos).into())
        }) {
            commands.entity(entity).with_child((
                Transform::from_xyz(0.0, 0.0, 2.0),
                ShapeBuilder::with(&path)
                    .stroke((Color::srgba(1.0, 0.0, 0.0, 0.3), 5.0))
                    .build(),
            ));
        }
    }
}
