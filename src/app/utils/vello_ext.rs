use std::collections::HashMap;
use crate::app::utils::debug::SoftExpect;
use bevy::prelude::*;
use bevy_vello::prelude::VelloScene2d;

pub struct VelloExtPlugin;

#[derive(Clone, Eq, Hash, PartialEq, Debug, SystemSet)]
pub struct VelloDraw;

impl Plugin for VelloExtPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, on_spawn_draw.in_set(VelloDraw));
    }
}

#[derive(Component)]
#[require(VelloScene2d)]
pub struct VelloEnhancedScene {
    pub on_layer_draw_begin: HashMap<isize, Box<dyn Fn(&mut VelloScene2d) + Send + Sync>>,
}

#[derive(Component, Reflect)]
#[relationship_target(relationship = VelloElementWithScene)]
pub struct VelloSceneWithElements(Vec<Entity>);

#[derive(Component, Reflect)]
#[relationship(relationship_target = VelloSceneWithElements)]
pub struct VelloElementWithScene(pub Entity);

#[derive(Component)]
pub struct VelloElement {
    pub layer: isize,
    pub on_draw: Box<dyn Fn(&mut VelloScene2d) + Send + Sync>,
}

fn on_spawn_draw(
    elements: Query<&VelloElement>,
    scenes: Query<(&mut VelloScene2d, Option<&VelloEnhancedScene>, &VelloSceneWithElements), Or<(Changed<VelloEnhancedScene>, Changed<VelloSceneWithElements>)>>,
) {
    for (mut scene, scene_enh, VelloSceneWithElements(scene_element_ids)) in scenes {
        scene.reset();

        let mut layer_elements = HashMap::new();

        if let Some(scene_enh) = scene_enh.as_ref() {
            for layer in scene_enh.on_layer_draw_begin.keys() {
                layer_elements.insert(*layer, Vec::new());
            }
        }

        for element in scene_element_ids
            .iter()
            .flat_map(|element_id| elements.get(*element_id).ok().soft_expect("")) {
            layer_elements.entry(element.layer).or_insert_with(Vec::new).push(element);
        }

        let mut layers = layer_elements.keys().copied().collect::<Vec<_>>();
        layers.sort_unstable();

        for layer in layers.into_iter() {
            if let Some(on_layer_draw_begin) = scene_enh.as_ref().and_then(|enh| enh.on_layer_draw_begin.get(&layer)) {
                (on_layer_draw_begin)(&mut scene);
            }

            if let Some(elements) = layer_elements.get(&layer) {
                for element in elements {
                    (element.on_draw)(&mut scene);
                }
            }
        }
    }
}
