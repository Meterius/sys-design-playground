use crate::app::common::settings::Settings;
use bevy::camera::primitives::Aabb;
use bevy::prelude::*;

pub struct DebugGizmosPlugin;

impl Plugin for DebugGizmosPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PostUpdate, draw_aabb_gizmos.run_if(Settings::in_debug_mode));
    }
}

#[derive(Component, Reflect)]
pub struct DebugGizmoColor(pub Color);

#[derive(Component, Reflect)]
pub struct DebugAabbGizmo;

fn draw_aabb_gizmos(
    items: Query<(&Aabb, &GlobalTransform, Option<&DebugGizmoColor>), With<DebugAabbGizmo>>,
    mut gizmos: Gizmos,
) {
    for (aabb, transform, color) in items.iter() {
        let color = color.map(|c| c.0).unwrap_or(Color::srgb(1.0, 0.0, 0.0));
        gizmos.aabb_3d(Aabb {
            center: aabb.center,
            half_extents: aabb.half_extents + Vec3A::ONE * 0.00001,
        }, *transform, color);
    }
}
