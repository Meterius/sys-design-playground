use crate::app::map::transform;
use crate::app::map::transform::MERCATOR_WORLD_SIZE;
use crate::app::maplibre_gl_js::integration::MaplibreMapIntegration;
use crate::app::maplibre_gl_js::types::MaplibreMapViewData;
use crate::utils::debug::SoftExpect;
use bevy::camera::CameraProjection;
use bevy::math::{DMat4, DQuat, DVec3, DVec4};
use bevy::prelude::*;
use big_space::prelude::{CellCoord, Grid};

const MAPLIBRE_DEFAULT_FOV_RADIANS: f64 = 0.643_501_108_793_284_4;
const DEFAULT_MAPLIBRE_FAR: f32 = 1_000.0;

pub(super) struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (sync_camera,).chain());
    }
}

#[derive(Debug, Reflect, Component)]
pub struct MapViewCamera {
    pub maplibre_int_id: Entity,
}

fn sync_camera(
    maps: Query<&MaplibreMapIntegration>,
    mut cameras: Query<(
        &MapViewCamera,
        &mut CellCoord,
        &mut Transform,
        &mut Projection,
        &ChildOf,
    )>,
    grids: Query<&Grid>,
) {
    for (camera, mut cell, mut transform, mut projection, child_of) in &mut cameras {
        let Some(map_int) = maps.get(camera.maplibre_int_id).ok().soft_expect("") else {
            continue;
        };

        let Some(grid) = grids.get(child_of.0).ok().soft_expect("") else {
            continue;
        };

        let semantic_camera = maplibre_semantic_camera_transform(&map_int.view);
        let (camera_cell, camera_translation) =
            grid.translation_to_grid(semantic_camera.translation);

        *cell = camera_cell;
        *transform = Transform::from_translation(camera_translation)
            .with_rotation(semantic_camera.rotation.as_quat());

        let Some(maplibre_projection) = MapLibreMercatorProjection::from_main_matrix(
            &map_int.view.main_matrix,
            semantic_camera.world_from_view,
        ) else {
            continue;
        };

        match projection.as_mut() {
            Projection::Custom(custom) => {
                if let Some(existing) = custom.get_mut::<MapLibreMercatorProjection>() {
                    *existing = maplibre_projection;
                } else {
                    *projection = Projection::custom(maplibre_projection);
                }
            }
            _ => {
                *projection = Projection::custom(maplibre_projection);
            }
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct SemanticCameraTransform {
    translation: DVec3,
    rotation: DQuat,
    world_from_view: DMat4,
}

pub fn maplibre_camera_to_center_distance_world(height: f64, zoom: f64) -> f64 {
    let world_size_pixels = 512.0 * 2.0_f64.powf(zoom);
    if height <= 0.0 || world_size_pixels <= 0.0 {
        return 0.0;
    }

    let camera_to_center_pixels = 0.5 * height / (MAPLIBRE_DEFAULT_FOV_RADIANS * 0.5).tan();
    camera_to_center_pixels / world_size_pixels * MERCATOR_WORLD_SIZE
}

fn maplibre_semantic_camera_transform(state: &MaplibreMapViewData) -> SemanticCameraTransform {
    let center = transform::lng_lat_to_world(state.center_lng, state.center_lat, 0.0);
    let pitch = state.pitch.to_radians();
    let bearing = state.bearing.to_radians();
    let rotation = DQuat::from_rotation_z(-bearing) * DQuat::from_rotation_x(pitch);
    let forward = rotation * -DVec3::Z;
    let distance = maplibre_camera_to_center_distance_world(state.height, state.zoom);
    let translation = center - forward * distance;

    SemanticCameraTransform {
        translation,
        rotation,
        world_from_view: DMat4::from_rotation_translation(rotation, translation),
    }
}

#[derive(Clone, Debug)]
struct MapLibreMercatorProjection {
    clip_from_view: Mat4,
    far: f32,
}

impl Default for MapLibreMercatorProjection {
    fn default() -> Self {
        Self {
            clip_from_view: Mat4::IDENTITY,
            far: DEFAULT_MAPLIBRE_FAR,
        }
    }
}

impl MapLibreMercatorProjection {
    fn from_main_matrix(main_matrix: &[f64], world_from_view: DMat4) -> Option<Self> {
        let main_matrix = main_matrix.try_into().ok()?;

        let maplibre_clip_from_mercator = DMat4::from_cols_array(main_matrix);
        let mercator_from_world = DMat4::from_cols(
            DVec4::new(1.0 / MERCATOR_WORLD_SIZE, 0.0, 0.0, 0.0),
            DVec4::new(0.0, -1.0 / MERCATOR_WORLD_SIZE, 0.0, 0.0),
            DVec4::new(0.0, 0.0, 1.0 / MERCATOR_WORLD_SIZE, 0.0),
            DVec4::W,
        );

        let clip_from_view = (opengl_to_wgpu_clip_matrix()
            * maplibre_clip_from_mercator
            * mercator_from_world
            * world_from_view)
            .as_mat4();

        Some(Self {
            clip_from_view,
            far: Self::far_from_clip_from_view(clip_from_view),
        })
    }

    fn unproject_ndc_to_view(&self, ndc_x: f32, ndc_y: f32, ndc_z: f32) -> Vec3 {
        self.clip_from_view
            .inverse()
            .project_point3(Vec3::new(ndc_x, ndc_y, ndc_z))
    }

    fn far_from_clip_from_view(clip_from_view: Mat4) -> f32 {
        let view_from_clip = clip_from_view.inverse();
        let far = [
            Vec3::new(1.0, -1.0, 1.0),
            Vec3::new(1.0, 1.0, 1.0),
            Vec3::new(-1.0, 1.0, 1.0),
            Vec3::new(-1.0, -1.0, 1.0),
        ]
        .into_iter()
        .map(|corner| view_from_clip.project_point3(corner).z.abs())
        .filter(|z| z.is_finite() && *z > 0.0)
        .fold(0.0, f32::max);

        if far.is_finite() && far > 0.0 {
            far
        } else {
            DEFAULT_MAPLIBRE_FAR
        }
    }

    fn frustum_corner_at_view_z(&self, ndc_x: f32, ndc_y: f32, view_z: f32) -> Vec3A {
        let near = self.unproject_ndc_to_view(ndc_x, ndc_y, 0.0);
        let far = self.unproject_ndc_to_view(ndc_x, ndc_y, 1.0);
        let ray = far - near;

        if !near.is_finite() || !far.is_finite() || ray.z.abs() <= f32::EPSILON {
            return Vec3A::from(near);
        }

        Vec3A::from(near + ray * ((view_z - near.z) / ray.z))
    }
}

impl CameraProjection for MapLibreMercatorProjection {
    fn get_clip_from_view(&self) -> Mat4 {
        self.clip_from_view
    }

    fn get_clip_from_view_for_sub(&self, _sub_view: &bevy::camera::SubCameraView) -> Mat4 {
        self.clip_from_view
    }

    fn update(&mut self, _width: f32, _height: f32) {}

    fn far(&self) -> f32 {
        self.far
    }

    fn get_frustum_corners(&self, z_near: f32, z_far: f32) -> [Vec3A; 8] {
        [
            self.frustum_corner_at_view_z(1.0, -1.0, z_near),
            self.frustum_corner_at_view_z(1.0, 1.0, z_near),
            self.frustum_corner_at_view_z(-1.0, 1.0, z_near),
            self.frustum_corner_at_view_z(-1.0, -1.0, z_near),
            self.frustum_corner_at_view_z(1.0, -1.0, z_far),
            self.frustum_corner_at_view_z(1.0, 1.0, z_far),
            self.frustum_corner_at_view_z(-1.0, 1.0, z_far),
            self.frustum_corner_at_view_z(-1.0, -1.0, z_far),
        ]
    }
}

fn opengl_to_wgpu_clip_matrix() -> DMat4 {
    DMat4::from_cols(
        DVec4::X,
        DVec4::Y,
        DVec4::new(0.0, 0.0, -0.5, 0.0),
        DVec4::new(0.0, 0.0, 0.5, 1.0),
    )
}
