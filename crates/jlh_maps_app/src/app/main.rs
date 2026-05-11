use crate::app::common::debug_gizmos::DebugGizmosPlugin;
use crate::app::common::external_render_target::{
    BevyRenderTextureConfig, ExternalRenderTargetPlugin,
};
use crate::app::common::settings::SettingsPlugin;
use crate::app::editor::EditorPlugin;
use crate::app::instance_management::InstanceManagementPlugin;
use crate::app::instance_management::instance::InstanceState;
use crate::app::instance_management::instance::register_instance;
use crate::app::map::MapPlugin;
use crate::app::map::core::{register_map_view_render_texture_reference, spawn_map_view};
use crate::app::maplibre_gl_js::MaplibreGlJsPlugin;
use crate::app::maplibre_gl_js::integration::MaplibreMapIntegration;
use bevy::log::LogPlugin;
use bevy::prelude::*;
use bevy::render::RenderPlugin;
use bevy::render::render_resource::WgpuFeatures;
use bevy::render::settings::{RenderCreation, WgpuSettings};
use bevy::window::{ExitCondition, PresentMode, Window, WindowPlugin, WindowResolution};
use bevy_inspector_egui::bevy_egui::EguiPlugin;
use bevy_winit::WinitSettings;
use big_space::plugin::BigSpaceDefaultPlugins;
use wasm_bindgen::prelude::{JsValue, wasm_bindgen};

#[wasm_bindgen]
pub fn initialize() {
    console_error_panic_hook::set_once();
}

#[wasm_bindgen]
pub fn mount(
    canvas_selector: String,
    render_texture_id: u32,
    render_texture_width: u32,
    render_texture_height: u32,
    external_framebuffer: JsValue,
    external_r32f_framebuffer: JsValue,
) {
    let instance_id = canvas_selector.clone();

    initialize();

    register_instance(canvas_selector.clone());

    let mut app = App::new();

    app.add_plugins((
        DefaultPlugins
            .set(WindowPlugin {
                primary_window: Some(Window {
                    present_mode: PresentMode::AutoVsync,
                    canvas: Some(canvas_selector),
                    fit_canvas_to_parent: true,
                    prevent_default_event_handling: false,
                    resolution: WindowResolution::default().with_scale_factor_override(1.0),
                    ..default()
                }),
                exit_condition: ExitCondition::DontExit,
                ..default()
            })
            .set(LogPlugin {
                filter: "info,wgpu_core=error,wgpu_hal=error".into(),
                ..default()
            })
            .set(RenderPlugin {
                render_creation: RenderCreation::Automatic(WgpuSettings {
                    features: WgpuFeatures::default(),
                    ..default()
                }),
                ..default()
            })
            .disable::<TransformPlugin>(),
        BigSpaceDefaultPlugins,
        EguiPlugin::default(),
        SettingsPlugin {},
        DebugGizmosPlugin,
        EditorPlugin {},
        MaplibreGlJsPlugin,
        MapPlugin,
        InstanceManagementPlugin { id: instance_id },
        ExternalRenderTargetPlugin {
            render_texture_id,
            render_texture_width,
            render_texture_height,
            external_framebuffer,
            external_r32f_framebuffer,
        },
    ));

    app.add_systems(PreUpdate, setup_map_for_integration);

    app.insert_resource(WinitSettings::continuous());

    app.run();
}

fn setup_map_for_integration(
    mut commands: Commands,
    instance: Res<InstanceState>,
    render_texture: Res<BevyRenderTextureConfig>,
    integrations: Query<(Entity, &MaplibreMapIntegration), Added<MaplibreMapIntegration>>,
) {
    for (int_entity, integration) in integrations.iter() {
        let reference = spawn_map_view(&mut commands, int_entity, render_texture.reference.clone());
        register_map_view_render_texture_reference(instance.id.clone(), integration.id, reference);
    }
}
