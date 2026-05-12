use crate::app::common::debug_gizmos::DebugGizmosPlugin;
use crate::app::common::editor::EditorPlugin;
use crate::app::common::external_render_target::{
    BevyRenderTextureConfig, ExternalRenderTargetPlugin,
};
use crate::app::common::materials::MaterialsPlugin;
use crate::app::common::settings::SettingsPlugin;
use crate::app::instance_management::InstanceManagementPlugin;
use crate::app::instance_management::instance::InstanceState;
use crate::app::instance_management::instance::register_instance;
use crate::app::instance_management::instance::unregister_instance;
use crate::app::manual_winit_runner::{prepare_manual_winit_app, spawn_manual_winit_runner};
use crate::app::map::MapPlugin;
use crate::app::map::core::{register_map_view_render_texture_reference, spawn_map_view};
use crate::app::maplibre_gl_js::MaplibreGlJsPlugin;
use crate::app::maplibre_gl_js::integration::MaplibreMapIntegration;
use bevy::app::PluginsState;
use bevy::log::LogPlugin;
use bevy::prelude::*;
use bevy::render::RenderPlugin;
use bevy::render::render_resource::WgpuFeatures;
use bevy::render::settings::{RenderCreation, WgpuSettings};
use bevy::window::{ExitCondition, PresentMode, Window, WindowPlugin, WindowResolution};
use bevy_inspector_egui::bevy_egui::EguiPlugin;
use bevy_winit::{WinitPlugin, WinitUserEvent};
use big_space::plugin::BigSpaceDefaultPlugins;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use tracing::info;
use wasm_bindgen::prelude::{JsValue, wasm_bindgen};
use winit::event_loop::EventLoop;

thread_local! {
    static BEVY_APPS: RefCell<HashMap<String, SharedBevyApp>> = RefCell::new(HashMap::new());
}

pub(crate) type SharedBevyApp = Rc<RefCell<ManagedBevyApp>>;

pub(crate) struct ManagedBevyApp {
    pub app: Option<App>,
    plugins_cleaned: bool,
}

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
    let event_loop = EventLoop::<WinitUserEvent>::with_user_event()
        .build()
        .expect("Failed to build event loop");

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
            .disable::<WinitPlugin>()
            .disable::<TransformPlugin>(),
        MaterialsPlugin,
        BigSpaceDefaultPlugins,
        EguiPlugin::default(),
        SettingsPlugin {},
        DebugGizmosPlugin,
        EditorPlugin {},
        MaplibreGlJsPlugin,
        MapPlugin,
        InstanceManagementPlugin {
            id: instance_id.clone(),
        },
        ExternalRenderTargetPlugin {
            render_texture_id,
            render_texture_width,
            render_texture_height,
            external_framebuffer,
            external_r32f_framebuffer,
        },
    ));

    app.add_systems(PreUpdate, setup_map_for_integration);
    prepare_manual_winit_app(&mut app, &event_loop);

    let shared_app = Rc::new(RefCell::new(ManagedBevyApp {
        app: Some(app),
        plugins_cleaned: false,
    }));
    BEVY_APPS.with(|apps| {
        apps.borrow_mut()
            .insert(instance_id.clone(), shared_app.clone());
    });

    spawn_manual_winit_runner(instance_id, event_loop, shared_app);
}

#[wasm_bindgen]
pub fn tick(instance_id: String) -> Result<(), String> {
    BEVY_APPS.with(|apps| {
        let apps = apps.borrow();
        let Some(managed_app) = apps.get(&instance_id) else {
            return Err(format!("Instance {instance_id} is not mounted"));
        };
        let mut managed_app = managed_app.borrow_mut();

        if !managed_app.plugins_cleaned {
            let Some(app) = managed_app.app.as_mut() else {
                return Err(format!("Instance {instance_id} is not mounted"));
            };
            if !finish_app_plugins_if_ready(app) {
                return Ok(());
            }
            managed_app.plugins_cleaned = true;
        }

        let Some(app) = managed_app.app.as_mut() else {
            return Err(format!("Instance {instance_id} is not mounted"));
        };
        app.update();
        Ok(())
    })
}

pub fn unmount_instance(instance_id: &str) -> Result<(), String> {
    BEVY_APPS.with(|apps| {
        let removed = apps
            .borrow_mut()
            .remove(instance_id)
            .map(|app| app.borrow_mut().app.take().is_some())
            .unwrap_or(false);
        unregister_instance(instance_id);

        if removed {
            info!("Unmounted Bevy instance {}", instance_id);
            Ok(())
        } else {
            Err(format!("Instance {instance_id} is not mounted"))
        }
    })
}

fn finish_app_plugins_if_ready(app: &mut App) -> bool {
    match app.plugins_state() {
        PluginsState::Adding => false,
        PluginsState::Ready => {
            app.finish();
            app.cleanup();
            true
        }
        PluginsState::Finished => {
            app.cleanup();
            true
        }
        PluginsState::Cleaned => true,
    }
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
