use crate::app::geo::map::MapViewContextQuery;
use crate::app::geo::map::MapViewContextRef;
use crate::app::utils::debug::SoftExpect;
use crate::app::utils::vello_ext::{VelloDraw, VelloElement, VelloElementWithScene};
use bevy::prelude::*;
use bevy_vello::prelude::{kurbo, peniko, VelloScene2d};
use glam::DVec2;
use itertools::Itertools;
use utilities::glam_ext::bounding::{AxisAlignedBoundingBox2D, DAabb2};
use crate::geo::coords::Projection2D;

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
    pub width: f32,
    pub color: Color,
    pub line: Vec<DVec2>,
}

impl VelloMapLine {
    pub fn new(scene_id: Entity, line: Vec<DVec2>, width: f32, color: Color) -> Self {
        Self {
            scene_id,
            width,
            color,
            line,
        }
    }
}

pub fn update_line(
    mut commands: Commands,
    view_context: MapViewContextQuery,
    scenes: Query<&Transform, With<VelloScene2d>>,
    lines: Query<(Entity, &VelloMapLine), Changed<VelloMapLine>>,
) {
    for (line_id, line) in lines {
        if let Some(ctx) = view_context.get(line_id).soft_expect("")
            && let Some(scene_transform) = scenes.get(line.scene_id).ok().soft_expect("")
        {
            commands.entity(line_id).insert((
                VelloElementWithScene(line.scene_id),
                VelloElement {
                    on_draw: {
                        let center_local = scene_transform.translation.xy().as_dvec2();
                        let color = peniko::Color::new(line.color.to_srgba().to_f32_array());

                        let width = line.width as f64;

                        let path = line.line.iter()
                            .map(|p| ctx.view.abs_to_local(ctx.map.projection.gcs_to_abs(*p)) - center_local)
                            .map(|p| kurbo::Point::new(p.x, p.y))
                            .enumerate()
                            .map(|(idx, p)| if idx == 0 {
                                kurbo::PathEl::MoveTo(p)
                            } else {
                                kurbo::PathEl::LineTo(p)
                            })
                            .chain(std::iter::once(kurbo::PathEl::ClosePath))
                            .collect_vec();

                        let path = kurbo::BezPath::from_vec(path);

                        Box::new(move |scene| {
                            scene.stroke(
                                &kurbo::Stroke::new(width),
                                kurbo::Affine::scale_non_uniform(1., -1.),
                                color,
                                None,
                                &path,
                            );
                        })
                    },
                },
            ));
        }
    }
}
