use crate::app::common::settings::Settings;
use crate::app::utils::debug::SoftExpect;
use crate::geo::coords::{BoundedMercatorProjection, Projection2D, approx_lat_delta_from_len};
use crate::utils::glam_ext::bounding::{Aabb2, AxisAlignedBoundingBox2D, DAabb2};
use bevy::ecs::query::QueryData;
use bevy::ecs::relationship::AncestorIter;
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy_pancam::{PanCam, PanCamClampBounds, PanCamSystems};
use bevy_vector_shapes::painter::ShapePainter;
use bevy_vector_shapes::prelude::LinePainter;
use bevy_vector_shapes::shapes::ThicknessType;
use big_space::grid::Grid;
use glam::{DAffine2, DVec2, dvec2};
use itertools::Itertools;
use std::path::Ancestors;

pub struct MapPlugin {}

impl Plugin for MapPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PostUpdate,
            (
                sync_map_view_transform.before(TransformSystems::Propagate),
                sync_view_from_camera.after(TransformSystems::Propagate),
                draw_map_view_debug,
            )
                .chain(),
        );
    }
}

#[derive(Component, Clone, Reflect)]
pub struct Map {
    pub projection: BoundedMercatorProjection,
}

#[derive(Component, Reflect)]
#[relationship_target(relationship = MapViewWithMap)]
pub struct MapWithViews(Vec<Entity>);

#[derive(Component, Clone, Reflect)]
pub struct MapView {
    pub scale: f64,
    pub viewport_abs: Option<DAabb2>,
    pub viewport_gcs: Option<DAabb2>,
}

impl MapView {
    pub fn new() -> Self {
        Self {
            scale: approx_lat_delta_from_len(10.0).recip(),
            viewport_abs: None,
            viewport_gcs: None,
        }
    }

    pub fn local_to_abs(&self, pos: DVec2) -> DVec2 {
        pos / self.scale
    }

    pub fn abs_to_local(&self, pos: DVec2) -> DVec2 {
        pos * self.scale
    }

    pub fn map_bounds_local(&self, map: &Map) -> DAabb2 {
        DAabb2::new(
            self.abs_to_local(map.projection.abs_bounds().min()),
            self.abs_to_local(map.projection.abs_bounds().max()),
        )
    }
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
                        .as_vec2()
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

#[derive(Component, Reflect)]
#[relationship(relationship_target = MapWithViews)]
pub struct MapViewWithMap(pub Entity);

#[derive(Component, Reflect)]
#[relationship_target(relationship = MapViewCameraWithView)]
pub struct MapViewWithCamera(Entity);

#[derive(Component, Reflect)]
pub struct MapViewCamera {}

#[derive(Component, Reflect)]
#[relationship(relationship_target = MapViewWithCamera)]
pub struct MapViewCameraWithView(pub Entity);

#[derive(SystemParam)]
pub struct MapViewContextQuery<'w, 's> {
    maps: Query<'w, 's, &'static Map>,
    views: Query<
        'w,
        's,
        (
            Entity,
            &'static Grid,
            &'static MapView,
            &'static MapViewWithMap,
        ),
    >,
    children: Query<'w, 's, &'static ChildOf>,
}

pub struct MapViewContext<'a> {
    pub map_id: Entity,
    pub map: &'a Map,

    pub view_id: Entity,
    pub view: &'a MapView,
    pub view_grid: &'a Grid,
}

impl<'w, 's> MapViewContextQuery<'w, 's> {
    pub fn get(&self, id: Entity) -> Option<MapViewContext> {
        let view = self
            .children
            .iter_ancestors::<ChildOf>(id)
            .filter_map(|p_id| self.views.get(p_id).ok())
            .next();

        view.and_then(move |(view_id, view_grid, view, &MapViewWithMap(map_id))| {
            self.maps.get(map_id).ok().map(move |map| MapViewContext {
                map,
                view_grid,
                view,
                map_id,
                view_id,
            })
        })
        .soft_expect("Could not construct view context")
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
                view.local_to_abs(
                    view_transform_inv
                        .transform_point3(cam_view_world_min)
                        .xy()
                        .as_dvec2(),
                ),
                view.local_to_abs(
                    view_transform_inv
                        .transform_point3(cam_view_world_max)
                        .xy()
                        .as_dvec2(),
                ),
            )
            .intersection(map.projection.abs_bounds());

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

#[derive(Component, Reflect)]
pub struct MapViewTransform {
    pub translation: DVec2,
}

fn sync_map_view_transform(
    transforms: Query<(Entity, &mut Transform, &MapViewTransform)>,
    parents: Query<&ChildOf>,
    views: Query<&MapView>,
) {
    for (tr_id, mut tr_transform, tr_view_transform) in transforms {
        if let Some(view) = parents
            .iter_ancestors(tr_id)
            .filter_map(|p_id| views.get(p_id).ok())
            .next()
            .soft_expect("")
        {
            tr_transform.translation = view
                .abs_to_local(tr_view_transform.translation)
                .as_vec2()
                .extend(tr_transform.translation.z);
        }
    }
}
