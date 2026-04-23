use crate::app::geo::map::MapViewContextQuery;
use crate::app::geo::map::MapViewContextRef;
use crate::app::utils::commands::InsertIfActive;
use crate::app::utils::debug::SoftExpect;
use crate::app::utils::vello_ext::{VelloDraw, VelloElement, VelloElementWithScene};
use crate::geo::coords::Projection2D;
use bevy::prelude::*;
use bevy_vello::prelude::{VelloScene2d, kurbo, peniko};
use glam::DVec2;
use itertools::Itertools;
use utilities::glam_ext::bounding::AxisAlignedBoundingBox2D;

pub struct VelloGeometryPlugin {}

impl Plugin for VelloGeometryPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, update_line.before(VelloDraw));
    }
}

#[derive(Component)]
#[require(MapViewContextRef)]
pub struct VelloMapLine {
    pub scene_id: Entity,
    pub scene_center_abs: DVec2,
    pub width: f32,
    pub color: Color,
    pub line: Vec<DVec2>,
}

impl VelloMapLine {
    pub fn new(
        scene_id: Entity,
        scene_center_abs: DVec2,
        line: Vec<DVec2>,
        width: f32,
        color: Color,
    ) -> Self {
        Self {
            scene_id,
            scene_center_abs,
            width,
            color,
            line,
        }
    }
}

pub fn update_line(
    mut commands: Commands,
    view_context: MapViewContextQuery,
    lines: Query<(Entity, &VelloMapLine), Changed<VelloMapLine>>,
) {
    for (line_id, line) in lines {
        if let Some(ctx) = view_context.get(line_id).soft_expect("") {
            let scene_id = line.scene_id;

            let on_draw = {
                let color = peniko::Color::new(line.color.to_srgba().to_f32_array());

                let width = line.width as f64;

                let path = line
                    .line
                    .iter()
                    .map(|p| {
                        ctx.view.abs_to_local(ctx.map.projection.gcs_to_abs(*p))
                            - ctx.view.abs_to_local(line.scene_center_abs)
                    })
                    .map(|p| kurbo::Point::new(p.x, p.y))
                    .enumerate()
                    .map(|(idx, p)| {
                        if idx == 0 {
                            kurbo::PathEl::MoveTo(p)
                        } else {
                            kurbo::PathEl::LineTo(p)
                        }
                    })
                    .collect_vec();

                let path = kurbo::BezPath::from_vec(path);

                Box::new(move |scene: &mut VelloScene2d| {
                    scene.stroke(
                        &kurbo::Stroke::new(width),
                        kurbo::Affine::scale_non_uniform(1., -1.),
                        color,
                        None,
                        &path,
                    );
                })
            };

            commands.queue(InsertIfActive {
                entity: line_id,
                bundle: (VelloElementWithScene(scene_id), VelloElement { on_draw }),
            });
        }
    }
}
