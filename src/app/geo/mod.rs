use crate::app::geo::boundaries::load_all_shape_paths;
use crate::app::geo::locations::LocationsPlugin;
use crate::app::geo::map::MapPlugin;
use crate::app::geo::tiling::GeoMapTilingPlugin;
use crate::geo::coords::{BoundedMercatorProjection, LonLatVec2, Projection2D, RadLonLatVec2};
use bevy::prelude::*;
use bevy_prototype_lyon::geometry::ShapeBuilderBase;
use bevy_prototype_lyon::path::ShapePath;
use bevy_prototype_lyon::prelude::ShapeBuilder;

pub mod boundaries;
pub mod locations;
pub mod map;
pub mod tiling;

pub struct GeoMapPlugin {}

impl Plugin for GeoMapPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (apply_transform, geo_map_plane_setup, handle_plane_view_sync),
        );
        app.add_plugins((
            MapPlugin {},
            GeoMapTilingPlugin::default(),
            LocationsPlugin::default(),
        ));
    }
}

#[derive(Component)]
pub struct GeoMapPlane {
    pub(crate) projection: BoundedMercatorProjection,
    pub(crate) scale: f32,
}

impl GeoMapPlane {
    fn local_to_abs(&self, pos: &Vec2) -> Vec2 {
        pos / self.scale + self.projection.abs_pos()
    }

    fn abs_to_local(&self, pos: &Vec2) -> Vec2 {
        (pos - self.projection.abs_pos()) * self.scale
    }
}

#[derive(Component)]
#[relationship(relationship_target = GeoMapElements)]
pub struct GeoMapElementOf(pub Entity);

#[derive(Component)]
#[relationship_target(relationship = GeoMapElementOf)]
pub struct GeoMapElements(Vec<Entity>);

#[derive(Default, Component)]
pub struct GeoMapPlaneView {
    pub view_gcs: Option<(RadLonLatVec2, RadLonLatVec2)>,
}

fn handle_plane_view_sync(
    mut planes: Query<(&GlobalTransform, &GeoMapPlane, &mut GeoMapPlaneView)>,
    camera: Query<(&GlobalTransform, &Camera)>,
) {
    if let Ok((camera_transform, camera)) = camera.single() {
        for (plane_transform, plane, mut plane_view) in planes.iter_mut() {
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
                let cam_abs_min = plane
                    .projection
                    .abs_to_gcs(&plane.local_to_abs(&(cam_global_min - plane_pos)));
                let cam_abs_max = plane
                    .projection
                    .abs_to_gcs(&plane.local_to_abs(&(cam_global_max - plane_pos)));

                plane_view.view_gcs = Some((cam_abs_min, cam_abs_max));
            }
        }
    }
}

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
                .abs_to_local(&plane.projection.gcs_to_abs(&geo_transform.pos))
                .extend(transform.translation.z);
        }
    }
}

fn geo_map_plane_setup(
    mut commands: Commands,
    planes: Query<(Entity, &GeoMapPlane), Added<GeoMapPlane>>,
    _meshes: ResMut<Assets<Mesh>>,
    _materials: ResMut<Assets<ColorMaterial>>,
    _asset_server: Res<AssetServer>,
) {
    for (entity, plane) in planes {
        let plane_size = plane.projection.abs_size() * plane.scale;

        let background_border = [
            Vec2::new(-1.0, -1.0) * plane_size / 2.0,
            Vec2::new(-1.0, 1.0) * plane_size / 2.0,
            Vec2::new(1.0, 1.0) * plane_size / 2.0,
            Vec2::new(1.0, -1.0) * plane_size / 2.0,
            Vec2::new(-1.0, -1.0) * plane_size / 2.0,
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
                .stroke((Color::BLACK, 5.0))
                .fill(Color::srgba(0.4, 0.4, 0.8, 0.25))
                .build(),
        ));

        for path in load_all_shape_paths("assets/datasets/geojson", |pos| {
            plane.abs_to_local(&plane.projection.gcs_to_abs(&LonLatVec2::from(pos).into()))
        }) {
            commands.entity(entity).with_child((
                Transform::from_xyz(0.0, 0.0, 10.0),
                ShapeBuilder::with(&path)
                    .stroke((Color::srgba(1.0, 0.0, 0.0, 0.3), 0.5))
                    .build(),
            ));
        }
    }
}
