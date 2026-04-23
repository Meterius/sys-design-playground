use bevy::prelude::*;
use bevy_vello::prelude::VelloScene2d;
use crate::app::utils::debug::SoftExpect;

pub struct VelloExtPlugin;

#[derive(Clone, Eq, Hash, PartialEq, Debug, SystemSet)]
pub struct VelloDraw;

impl Plugin for VelloExtPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, on_spawn_draw.in_set(VelloDraw));
    }
}

#[derive(Component, Reflect)]
#[relationship_target(relationship = VelloElementWithScene)]
pub struct VelloSceneWithElements(Vec<Entity>);

#[derive(Component, Reflect)]
#[relationship(relationship_target = VelloSceneWithElements)]
pub struct VelloElementWithScene(pub Entity);

#[derive(Component)]
pub struct VelloElement {
    pub on_draw: Box<dyn Fn(&mut VelloScene2d) + Send + Sync>
}

fn on_spawn_draw(
    elements: Query<&VelloElement>,
    scenes: Query<(&mut VelloScene2d, &VelloSceneWithElements), Changed<VelloSceneWithElements>>,
) {
    for (mut scene, VelloSceneWithElements(scene_element_ids)) in scenes {
        scene.reset();

        for &element_id in scene_element_ids {
            if let Some(element) = elements.get(element_id).ok().soft_expect("") {
                (element.on_draw)(&mut scene);
            }
        }
    }
}