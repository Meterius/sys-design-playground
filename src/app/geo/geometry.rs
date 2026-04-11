use crate::app::geo::map::MapViewContextQuery;
use crate::geo::coords::Projection2D;
use bevy::prelude::*;
use bevy_prototype_lyon::path::ShapePath;
use bevy_prototype_lyon::prelude::{ShapeBuilder, ShapeBuilderBase};
use bevy_prototype_lyon::shapes;
use glam::DVec2;
use itertools::Itertools;
use utilities::glam_ext::bounding::{AxisAlignedBoundingBox2D, DAabb2};

pub struct GeometryPlugin {}

impl Plugin for GeometryPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PostUpdate, (update_line, update_region));
    }
}

#[derive(Component)]
pub struct MapRegion {
    pub color: Color,
    pub perimeter: Vec<DVec2>,
    bounds: DAabb2,
}

impl MapRegion {
    pub fn new(perimeter: Vec<DVec2>, color: Color) -> Self {
        Self {
            color,
            bounds: DAabb2::new(
                perimeter
                    .iter()
                    .cloned()
                    .reduce(|prev, val| prev.min(val))
                    .unwrap(),
                perimeter
                    .iter()
                    .cloned()
                    .reduce(|prev, val| prev.max(val))
                    .unwrap(),
            ),
            perimeter,
        }
    }
}

pub fn update_region(
    mut commands: Commands,
    view_context: MapViewContextQuery,
    regions: Query<(Entity, &Transform, &MapRegion), Changed<MapRegion>>,
) {
    for (reg_id, reg_transform, reg) in regions {
        if let Some(ctx) = view_context.get(reg_id) {
            let center_local = DAabb2::new(
                ctx.map.projection.gcs_to_abs(reg.bounds.min()),
                ctx.map.projection.gcs_to_abs(reg.bounds.max()),
            )
            .center();

            let (center_cell, center_rel) = ctx
                .view_grid
                .translation_to_grid(center_local.extend(reg_transform.translation.z as f64));

            commands
                .entity(reg_id)
                .insert((center_cell, Transform::from_translation(center_rel)));

            commands.entity(reg_id).insert(
                ShapeBuilder::with(&shapes::Polygon {
                    points: reg
                        .perimeter
                        .iter()
                        .map(|pos| {
                            (ctx.view.abs_to_local(ctx.map.projection.gcs_to_abs(*pos))
                                - center_local)
                                .as_vec2()
                        })
                        .collect_vec(),
                    closed: true,
                })
                .fill(reg.color)
                .build(),
            );
        }
    }
}

#[derive(Component)]
pub struct MapLine {
    pub width: f32,
    pub color: Color,
    pub line: Vec<DVec2>,
    bounds: DAabb2,
}

impl MapLine {
    pub fn new(line: Vec<DVec2>, width: f32, color: Color) -> Self {
        Self {
            width,
            color,
            bounds: DAabb2::new(
                line.iter()
                    .cloned()
                    .reduce(|prev, val| prev.min(val))
                    .unwrap(),
                line.iter()
                    .cloned()
                    .reduce(|prev, val| prev.max(val))
                    .unwrap(),
            ),
            line,
        }
    }
}

pub fn update_line(
    mut commands: Commands,
    view_context: MapViewContextQuery,
    regions: Query<(Entity, &Transform, &MapLine), Changed<MapLine>>,
) {
    for (reg_id, reg_transform, reg) in regions {
        if let Some(ctx) = view_context.get(reg_id) {
            let center_local = DAabb2::new(
                ctx.map.projection.gcs_to_abs(reg.bounds.min()),
                ctx.map.projection.gcs_to_abs(reg.bounds.max()),
            )
            .center();

            let (center_cell, center_rel) = ctx
                .view_grid
                .translation_to_grid(center_local.extend(reg_transform.translation.z as f64));

            commands
                .entity(reg_id)
                .insert((center_cell, Transform::from_translation(center_rel)));

            let mut path = ShapePath::new();
            path = path.move_to(
                (ctx.view
                    .abs_to_local(ctx.map.projection.gcs_to_abs(reg.line[0]))
                    - center_local)
                    .as_vec2(),
            );

            for &pos in reg.line.iter() {
                path = path.line_to(
                    (ctx.view.abs_to_local(ctx.map.projection.gcs_to_abs(pos)) - center_local)
                        .as_vec2(),
                );
            }

            commands.entity(reg_id).insert(
                ShapeBuilder::with(&path)
                    .stroke((reg.color, reg.width))
                    .build(),
            );
        }
    }
}
