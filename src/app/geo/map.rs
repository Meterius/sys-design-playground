use crate::app::settings::Settings;
use crate::app::utils::SoftExpect;
use crate::geo::coords::{BoundedMercatorProjection, Projection2D};
use crate::geo::sub_division::{SubDivision2d, TileKey};
use crate::utils::glam_ext::bounding::{Aabb2, AxisAlignedBoundingBox2D, DAabb2};
use bevy::camera::CameraProjection;
use bevy::input::ButtonState;
use bevy::input::keyboard::KeyboardInput;
use bevy::math::USizeVec2;
use bevy::math::bounding::{Aabb2d, BoundingVolume};
use bevy::prelude::*;
use bevy_inspector_egui::bevy_egui::{EguiContexts, EguiPrimaryContextPass};
use bevy_pancam::{PanCam, PanCamClampBounds, PanCamSystems};
use bevy_prototype_lyon::draw::{Fill, Stroke};
use bevy_prototype_lyon::prelude::{Shape, ShapeBuilder, ShapeBuilderBase};
use bevy_prototype_lyon::shapes;
use bevy_prototype_lyon::shapes::RectangleOrigin;
use bevy_vector_shapes::painter::ShapePainter;
use bevy_vector_shapes::prelude::{DiscPainter, LinePainter, RectPainter, ShapeBundle};
use bevy_vector_shapes::shapes::ThicknessType;
use glam::{DAffine2, DAffine3, DMat2, DVec2, dvec2};
use std::collections::hash_map::Entry;
use std::collections::{HashMap, HashSet};

pub struct MapPlugin {}

impl Plugin for MapPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (reposition_view, adjust_pan_cam_bounds)
                .chain()
                .before(PanCamSystems),
        );

        app.add_systems(
            PostUpdate,
            (
                sync_view_from_camera.after(TransformSystems::Propagate),
                draw_map_view_debug,
            )
                .chain(),
        );
    }
}

#[derive(Component)]
pub struct Map {
    pub projection: BoundedMercatorProjection,
}

#[derive(Component)]
#[relationship_target(relationship = MapViewWithMap)]
pub struct MapWithViews(Vec<Entity>);

#[derive(Component, Default, Reflect)]
pub struct MapView {
    abs_transform: DAffine2,

    pub viewport_abs: Option<DAabb2>,
    pub viewport_gcs: Option<DAabb2>,
}

impl MapView {
    pub fn local_to_abs(&self, pos: Vec2) -> DVec2 {
        self.abs_transform
            .inverse()
            .transform_point2(pos.as_dvec2())
    }

    pub fn abs_to_local(&self, pos: DVec2) -> Vec2 {
        self.abs_transform.transform_point2(pos).as_vec2()
    }
}

#[derive(EntityEvent)]
pub struct MapViewAbsLocalTransformChanged {
    #[event_target]
    view_id: Entity,
}

fn draw_map_view_debug(
    views: Query<(&GlobalTransform, &MapView, &MapViewWithMap)>,
    maps: Query<&Map>,
    mut painter: ShapePainter,
    settings: Res<Settings>,
) {
    if settings.debug_mode {
        for (view_transform, view, &MapViewWithMap(map_id)) in views {
            if let Some(map) = maps.get(map_id).ok().soft_expect("") {
                painter.thickness_type = ThicknessType::Pixels;
                painter.thickness = 2.0;

                let gcs_to_world = |pos: DVec2| {
                    view_transform.transform_point(
                        view.abs_to_local(
                            map.projection.gcs_to_abs(
                                map.projection
                                    .gcs_bounds()
                                    .closest_point(pos.map(f64::to_degrees)),
                            ),
                        )
                        .extend(0.0),
                    )
                };

                painter.color = Color::srgb(1.0, 0.0, 0.0);
                for lat in -9..=9 {
                    let lat = lat as f64 * 10.0;
                    painter.line(
                        gcs_to_world(dvec2(-180.0, lat)),
                        gcs_to_world(dvec2(180.0, lat)),
                    );
                }

                for lon in -18..=18 {
                    let lon = lon as f64 * 10.0;
                    painter.line(
                        gcs_to_world(dvec2(lon, -90.0)),
                        gcs_to_world(dvec2(lon, 90.0)),
                    );
                }
            }
        }
    }
}

#[derive(Component)]
#[relationship(relationship_target = MapWithViews)]
pub struct MapViewWithMap(pub Entity);

#[derive(Component)]
#[relationship_target(relationship = MapViewCameraWithView)]
pub struct MapViewWithCamera(Entity);

#[derive(Component)]
pub struct MapViewCamera {}

#[derive(Component)]
#[relationship(relationship_target = MapViewWithCamera)]
pub struct MapViewCameraWithView(pub Entity);

fn adjust_pan_cam_bounds(
    mut commands: Commands,
    view_cameras: Query<
        (Entity, &mut PanCam, &mut Projection, &MapViewCameraWithView),
        With<MapViewCamera>,
    >,
    mut views: Query<(&GlobalTransform, &mut MapView, &MapViewWithMap)>,
    maps: Query<&Map>,
) {
    for (cam_id, mut pan_cam, mut cam_proj, &MapViewCameraWithView(view_id)) in view_cameras {
        if let Some((view_transform, mut view, &MapViewWithMap(map_id))) =
            views.get_mut(view_id).ok().soft_expect("")
            && let Some(map) = maps.get(map_id).ok().soft_expect("")
        {
            let view_world_bounds = Aabb2::new(
                view_transform
                    .transform_point(
                        view.abs_to_local(map.projection.abs_bounds().min())
                            .extend(0.0),
                    )
                    .xy(),
                view_transform
                    .transform_point(
                        view.abs_to_local(map.projection.abs_bounds().max())
                            .extend(0.0),
                    )
                    .xy(),
            );

            if let Projection::Orthographic(cam_proj) = &mut *cam_proj {
                let max_scale = (view_world_bounds.size()
                    / (cam_proj.area.size() / cam_proj.scale))
                    .min_element();

                if max_scale != pan_cam.max_scale {
                    pan_cam.max_scale = max_scale;
                    cam_proj.scale = cam_proj.scale.min(pan_cam.max_scale);
                    commands.trigger(PanCamClampBounds { entity: cam_id });
                }

                let pan_cam_bounds = Aabb2::new(
                    Vec2::new(pan_cam.min_x, pan_cam.min_y),
                    Vec2::new(pan_cam.max_x, pan_cam.max_x),
                );

                if pan_cam_bounds != view_world_bounds {
                    pan_cam.min_x = view_world_bounds.min().x;
                    pan_cam.min_y = view_world_bounds.min().y;
                    pan_cam.max_x = view_world_bounds.max().x;
                    pan_cam.max_y = view_world_bounds.max().y;

                    commands.trigger(PanCamClampBounds { entity: cam_id });
                }
            } else {
                warn!("Expected orthographic camera projection");
            }
        }
    }
}

fn sync_view_from_camera(
    view_cameras: Query<(&GlobalTransform, &Camera, &MapViewCameraWithView), With<MapViewCamera>>,
    mut views: Query<(&GlobalTransform, &mut MapView, &MapViewWithMap)>,
    maps: Query<&Map>,
) {
    for (cam_transform, cam, &MapViewCameraWithView(view_id)) in view_cameras {
        if let Some((view_transform, mut view, &MapViewWithMap(map_id))) =
            views.get_mut(view_id).ok().soft_expect("")
            && let Some(map) = maps.get(map_id).ok().soft_expect("")
            && let Some(cam_view_world_min) = cam
                .ndc_to_world(cam_transform, -Vec2::ONE.extend(0.0))
                .soft_expect("")
            && let Some(cam_view_world_max) = cam
                .ndc_to_world(cam_transform, Vec2::ONE.extend(0.0))
                .soft_expect("")
        {
            let view_transform_inv = view_transform.affine().inverse();

            let cam_view_abs = DAabb2::new(
                view.local_to_abs(view_transform_inv.transform_point3(cam_view_world_min).xy()),
                view.local_to_abs(view_transform_inv.transform_point3(cam_view_world_max).xy()),
            )
            .intersection(map.projection.abs_bounds())
            .soft_expect("");

            view.viewport_gcs = cam_view_abs.as_ref().map(|cam_view_abs| {
                DAabb2::new(
                    map.projection.abs_to_gcs(cam_view_abs.min()),
                    map.projection.abs_to_gcs(cam_view_abs.max()),
                )
            });
            view.viewport_abs = cam_view_abs;
        }
    }
}

pub fn reposition_view(
    mut view_cameras: Query<
        (
            &GlobalTransform,
            &mut Transform,
            &Camera,
            &mut Projection,
            &MapViewCameraWithView,
        ),
        With<MapViewCamera>,
    >,
    mut views: Query<(&GlobalTransform, &mut MapView, &MapViewWithMap)>,
    maps: Query<&Map>,
) {
    for (cam_transform_g, mut cam_transform, cam, mut cam_proj, &MapViewCameraWithView(view_id)) in
        view_cameras
    {
        if let Some((view_transform, mut view, &MapViewWithMap(map_id))) =
            views.get_mut(view_id).ok().soft_expect("")
            && let Some(map) = maps.get(map_id).ok().soft_expect("")
            && let Some(cam_center_world) = cam
                .ndc_to_world(cam_transform_g, Vec3::ZERO)
                .soft_expect("")
        {
            let origin_local = view_transform
                .affine()
                .inverse()
                .transform_point3(Vec3::ZERO)
                .xy();
            let origin_abs = view.local_to_abs(origin_local);

            let cam_center_local = view_transform
                .affine()
                .inverse()
                .transform_point3(cam_center_world)
                .xy();
            let cam_center_abs = view.local_to_abs(cam_center_local);

            let reposition = match cam_proj.as_ref() {
                Projection::Orthographic(cam_proj) => {
                    (cam_transform.translation.xy() / cam_proj.scale)
                        .max_element()
                        .abs()
                        >= 10000.0
                        || cam_proj.scale.log2().abs() >= 12.0
                }
                _ => false,
            };

            if reposition {
                view.abs_transform.translation -= cam_center_abs - origin_abs;
                cam_transform.translation = Vec3::ZERO;

                if let Projection::Orthographic(cam_proj) = cam_proj.as_mut() {
                    view.abs_transform.matrix2 /= cam_proj.scale as f64;
                    view.abs_transform.translation /= cam_proj.scale as f64;
                    cam_proj.scale = 1.0;
                }
            }
        }
    }
}
