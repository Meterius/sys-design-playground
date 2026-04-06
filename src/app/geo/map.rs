use crate::app::settings::Settings;
use crate::app::utils::SoftExpect;
use crate::geo::coords::{BoundedMercatorProjection, LonLatVec2, Projection2D, RadLonLatVec2};
use crate::geo::sub_division::{SubDivision2d, TileKey};
use crate::utils::{Aabb2dFromCorners, Aabb2dIntersect, Aabb2dSized};
use bevy::math::USizeVec2;
use bevy::math::bounding::{Aabb2d, BoundingVolume};
use bevy::prelude::*;
use bevy_pancam::{PanCam, PanCamClampBounds, PanCamSystems};
use bevy_prototype_lyon::draw::{Fill, Stroke};
use bevy_prototype_lyon::prelude::{Shape, ShapeBuilder, ShapeBuilderBase};
use bevy_prototype_lyon::shapes;
use bevy_prototype_lyon::shapes::RectangleOrigin;
use bevy_vector_shapes::painter::ShapePainter;
use bevy_vector_shapes::prelude::{DiscPainter, LinePainter, RectPainter, ShapeBundle};
use bevy_vector_shapes::shapes::ThicknessType;
use std::collections::hash_map::Entry;
use std::collections::{HashMap, HashSet};

pub struct MapPlugin {}

impl Plugin for MapPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                (sync_tiles_for_view, setup_tiles).chain(),
                adjust_pan_cam_bounds.before(PanCamSystems),
                draw_tiles_debug,
            ),
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

#[derive(Component, Default)]
pub struct MapView {
    viewport_abs: Option<Aabb2d>,
    viewport_gcs: Option<Aabb2d>,
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

                let gcs_to_world = |pos: RadLonLatVec2| {
                    view_transform.transform_point(
                        view.abs_to_local(map.projection.gcs_to_abs(&RadLonLatVec2::from(
                            map.projection.gcs_bbox().closest_point(Vec2::from(pos)),
                        )))
                        .extend(0.0),
                    )
                };

                painter.color = Color::srgb(1.0, 0.0, 0.0);
                for lat in -9..=9 {
                    let lat = lat as f32 * 10.0;
                    painter.line(
                        gcs_to_world(RadLonLatVec2::from(LonLatVec2 { x: -180.0, y: lat })),
                        gcs_to_world(RadLonLatVec2::from(LonLatVec2 { x: 180.0, y: lat })),
                    );
                }

                for lon in -18..=18 {
                    let lon = lon as f32 * 10.0;
                    painter.line(
                        gcs_to_world(RadLonLatVec2::from(LonLatVec2 { x: lon, y: -90.0 })),
                        gcs_to_world(RadLonLatVec2::from(LonLatVec2 { x: lon, y: 90.0 })),
                    );
                }
            }
        }
    }
}

impl MapView {
    pub fn local_to_abs(&self, pos: Vec2) -> Vec2 {
        pos
    }

    pub fn abs_to_local(&self, pos: Vec2) -> Vec2 {
        pos
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
            let view_world_bounds = Aabb2d::from_corners(
                view_transform
                    .transform_point(view.abs_to_local(map.projection.abs_bbox().min).extend(0.0))
                    .xy(),
                view_transform
                    .transform_point(view.abs_to_local(map.projection.abs_bbox().max).extend(0.0))
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

                let pan_cam_bounds = Aabb2d {
                    min: Vec2::new(pan_cam.min_x, pan_cam.min_y),
                    max: Vec2::new(pan_cam.max_x, pan_cam.max_x),
                };

                if pan_cam_bounds != view_world_bounds {
                    pan_cam.min_x = view_world_bounds.min.x;
                    pan_cam.min_y = view_world_bounds.min.y;
                    pan_cam.max_x = view_world_bounds.max.x;
                    pan_cam.max_y = view_world_bounds.max.y;

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

            let cam_view_abs = Aabb2d::from_corners(
                view.local_to_abs(view_transform_inv.transform_point3(cam_view_world_min).xy()),
                view.local_to_abs(view_transform_inv.transform_point3(cam_view_world_max).xy()),
            )
            .intersect(&map.projection.abs_bbox())
            .soft_expect("");

            view.viewport_gcs = cam_view_abs.as_ref().map(|cam_view_abs| {
                Aabb2d::from_corners(
                    Vec2::from(map.projection.abs_to_gcs(&cam_view_abs.min)),
                    Vec2::from(map.projection.abs_to_gcs(&cam_view_abs.max)),
                )
            });
            view.viewport_abs = cam_view_abs;
        }
    }
}

#[derive(Component)]
pub struct MapViewTiling {
    target_tile_count: usize,
    tiles: HashMap<TileKey, Entity>,
}

impl MapViewTiling {
    pub fn new(target_tile_count: usize) -> Self {
        Self {
            target_tile_count,
            tiles: HashMap::new(),
        }
    }
}

fn sync_tiles_for_view(
    mut commands: Commands,
    tilings: Query<(&mut MapViewTiling, &MapViewTilingWithView)>,
    views: Query<(Entity, &MapView, &MapViewWithMap)>,
    maps: Query<&Map>,
) {
    for (mut tiling, &MapViewTilingWithView(view_id)) in tilings {
        if let Some((view_id, view, &MapViewWithMap(map_id))) =
            views.get(view_id).ok().soft_expect("")
            && let Some(map) = maps.get(map_id).ok().soft_expect("")
            && let Some(viewport_abs) = view.viewport_abs
        {
            let abs_bbox = map.projection.abs_bbox();
            let sub_div = SubDivision2d { area: abs_bbox };

            let target_depth = sub_div.min_depth_for_tile_count(
                viewport_abs.size(),
                USizeVec2::new(tiling.target_tile_count, tiling.target_tile_count),
            );

            let mut required_tile_keys = HashSet::new();

            for tile in sub_div.tile_covering((viewport_abs.min, viewport_abs.max), target_depth) {
                required_tile_keys.insert(tile.key.clone());

                if let Entry::Vacant(mut entry) = tiling.tiles.entry(tile.key.clone()) {
                    let tile_id = commands
                        .spawn(MapViewTile {
                            area_abs: Aabb2d {
                                min: tile.bb_min,
                                max: tile.bb_max,
                            },
                            key: tile.key.clone(),
                        })
                        .id();
                    commands.entity(view_id).add_child(tile_id);
                    entry.insert(tile_id);
                }
            }

            for (_, tile_id) in tiling
                .tiles
                .extract_if(|key, _| !required_tile_keys.contains(key))
            {
                // commands.entity(tile_id).despawn();
            }
        }
    }
}

#[derive(Component)]
#[relationship_target(relationship = MapViewTilingWithView)]
pub struct MapViewWithTilings(Vec<Entity>);

#[derive(Component)]
#[relationship(relationship_target = MapViewWithTilings)]
pub struct MapViewTilingWithView(pub Entity);

#[derive(Component)]
pub struct MapViewTile {
    pub key: TileKey,
    pub area_abs: Aabb2d,
}

fn draw_tiles_debug(
    tiles: Query<(Entity, &MapViewTile, &ChildOf), Added<MapViewTile>>,
    views: Query<(&GlobalTransform, &MapView)>,
    mut painter: ShapePainter,
) {
    for (tile_id, tile, &ChildOf(view_id)) in tiles {
        if let Some((view_transform, view)) = views.get(view_id).ok().soft_expect("") {
            let area_world = (
                view_transform.transform_point(view.abs_to_local(tile.area_abs.min).extend(0.0)),
                view_transform.transform_point(view.abs_to_local(tile.area_abs.max).extend(0.0)),
            );

            painter.set_translation(((area_world.1 - area_world.0) / 2.0).xy().extend(10.0));
            painter.color = Color::srgb(0.0, 1.0, 0.0);
            painter.rect((area_world.1 - area_world.0).xy());
        }
    }
}

fn setup_tiles(
    mut commands: Commands,
    added_tiles: Query<(Entity, &MapViewTile, &ChildOf), Added<MapViewTile>>,
    views: Query<&MapView>,
) {
    for (tile_id, tile, &ChildOf(view_id)) in added_tiles {
        if let Some(view) = views.get(view_id).ok().soft_expect("") {
            let area_local = Aabb2d::from_corners(
                view.abs_to_local(tile.area_abs.min),
                view.abs_to_local(tile.area_abs.max),
            );
            info!("{area_local:?}");
            let mut tile_commands = commands.entity(tile_id);

            tile_commands.insert((
                Transform::from_translation(area_local.center().extend(10.0)),
                Visibility::default(),
            ));

            tile_commands.with_child(
                (ShapeBuilder::with(&shapes::Rectangle {
                    extents: area_local.size(),
                    ..default()
                })
                .fill(Color::BLACK)
                .stroke((Color::BLACK, area_local.size().max_element() * 0.01))
                .build()),
            );
        }
    }
}
