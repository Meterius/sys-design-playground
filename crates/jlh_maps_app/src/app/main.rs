use crate::app::common::debug_gizmos::DebugGizmosPlugin;
use crate::app::common::editor::EditorPlugin;
use crate::app::common::materials::MaterialsPlugin;
use crate::app::common::settings::SettingsPlugin;
use crate::app::instance_management::InstanceManagementPlugin;
use crate::app::instance_management::instance::{register_instance, unregister_instance};
use crate::app::map::MapPlugin;
use crate::app::map::core::spawn_map_view;
use crate::app::maplibre_gl_js::MaplibreGlJsPlugin;
use crate::app::maplibre_gl_js::integration::MaplibreMapIntegration;
use crate::app::offscreen_window_handle::OffscreenWindowHandle;
use bevy::app::PluginsState;
use bevy::input::ButtonState;
use bevy::input::keyboard::{Key, KeyCode, KeyboardInput, NativeKey, NativeKeyCode};
use bevy::input::mouse::{MouseButtonInput, MouseMotion, MouseScrollUnit, MouseWheel};
use bevy::light::DirectionalLightShadowMap;
use bevy::log::LogPlugin;
use bevy::prelude::*;
use bevy::render::extract_resource::{ExtractResource, ExtractResourcePlugin};
use bevy::render::render_resource::WgpuFeatures;
use bevy::render::settings::{Backends, RenderCreation, WgpuSettings};
use bevy::render::view::ExtractedWindows;
use bevy::render::{Render, RenderApp};
use bevy::render::{RenderPlugin, RenderSystems};
use bevy::window::{
    CompositeAlphaMode, CursorEntered, CursorLeft, CursorMoved, ExitCondition, PresentMode,
    PrimaryWindow, RawHandleWrapper, Window, WindowEvent as BevyWindowEvent, WindowFocused,
    WindowPlugin, WindowResized, WindowResolution, WindowScaleFactorChanged, WindowWrapper,
};
use bevy_inspector_egui::bevy_egui::{EguiGlobalSettings, EguiPlugin};
use bevy_winit::WinitPlugin;
use big_space::plugin::BigSpaceDefaultPlugins;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use tracing::info;
use wasm_bindgen::prelude::wasm_bindgen;
use web_sys::OffscreenCanvas;

thread_local! {
    static BEVY_APPS: RefCell<HashMap<String, SharedBevyApp>> = RefCell::new(HashMap::new());
}

pub(crate) type SharedBevyApp = Rc<RefCell<ManagedBevyApp>>;

pub(crate) struct ManagedBevyApp {
    pub app: Option<App>,
    plugins_cleaned: bool,
}

#[derive(Clone, Resource)]
pub struct AppWindows {
    pub debug: Option<Entity>,
    pub texture: Option<Entity>,
}

#[derive(Resource)]
pub struct OffscreenCanvases {
    pub debug: OffscreenCanvas,
    pub texture: OffscreenCanvas,
}

impl ExtractResource for AppWindows {
    type Source = Self;

    fn extract_resource(source: &Self::Source) -> Self {
        source.clone()
    }
}

#[wasm_bindgen]
pub fn initialize() {
    console_error_panic_hook::set_once();
}

#[wasm_bindgen]
pub fn mount(instance_id: String, debug_canvas: OffscreenCanvas, texture_canvas: OffscreenCanvas) {
    initialize();
    register_instance(instance_id.clone());

    let mut app = App::new();
    app.add_plugins((
        DefaultPlugins
            .set(WindowPlugin {
                primary_window: Some(Window {
                    canvas: None,
                    title: "Debug Offscreen Window".to_string(),
                    resolution: WindowResolution::new(debug_canvas.width(), debug_canvas.height()),
                    present_mode: PresentMode::AutoNoVsync,
                    transparent: true,
                    composite_alpha_mode: CompositeAlphaMode::PreMultiplied,
                    ..default()
                }),
                exit_condition: ExitCondition::DontExit,
                ..default()
            })
            .set(LogPlugin {
                filter: "info,wgpu_core=warn,wgpu_hal=warn".into(),
                ..default()
            })
            .set(RenderPlugin {
                render_creation: RenderCreation::Automatic(WgpuSettings {
                    features: WgpuFeatures::default(),
                    backends: Some(Backends::BROWSER_WEBGPU),
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
        ExtractResourcePlugin::<AppWindows>::default(),
    ));

    if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
        render_app.add_systems(
            Render,
            release_inactive_debug_window_surface
                .in_set(RenderSystems::Render)
                .after(bevy::render::renderer::render_system),
        );
    }

    app.insert_resource(ClearColor(Color::NONE));

    app.insert_resource(DirectionalLightShadowMap { size: 4096 });

    app.insert_resource(EguiGlobalSettings {
        // requires winit which is disabled as windows need manual management
        enable_ime: false,
        ..default()
    });

    app.insert_resource(AppWindows {
        debug: None,
        texture: None,
    });

    app.insert_non_send_resource(OffscreenCanvases {
        debug: debug_canvas,
        texture: texture_canvas,
    });

    app.add_systems(PreStartup, setup_offscreen_windows);
    app.add_systems(PreUpdate, setup_map_for_integration);

    let shared_app = Rc::new(RefCell::new(ManagedBevyApp {
        app: Some(app),
        plugins_cleaned: false,
    }));

    BEVY_APPS.with(|apps| {
        apps.borrow_mut().insert(instance_id, shared_app);
    });
}

fn raw_handle(canvas: &OffscreenCanvas) -> RawHandleWrapper {
    RawHandleWrapper::new(&WindowWrapper::new(OffscreenWindowHandle::new(canvas))).expect(
        "to create offscreen raw handle wrapper. If this fails, multiple threads are trying to access the same canvas!",
    )
}

fn setup_offscreen_windows(
    mut commands: Commands,
    canvases: NonSend<OffscreenCanvases>,
    mut app_windows: ResMut<AppWindows>,
    primary_windows: Query<Entity, (Added<Window>, With<PrimaryWindow>)>,
) {
    if app_windows.debug.is_none()
        && let Some(entity) = primary_windows.iter().next()
    {
        commands.entity(entity).insert(raw_handle(&canvases.debug));
        app_windows.debug = Some(entity);
    }

    if app_windows.texture.is_none() {
        let entity = commands
            .spawn((
                Window {
                    canvas: None,
                    title: "Map Texture Offscreen Window".to_string(),
                    resolution: WindowResolution::new(
                        canvases.texture.width(),
                        canvases.texture.height(),
                    ),
                    present_mode: PresentMode::AutoNoVsync,
                    transparent: true,
                    composite_alpha_mode: CompositeAlphaMode::PreMultiplied,
                    ..default()
                },
                raw_handle(&canvases.texture),
            ))
            .id();
        app_windows.texture = Some(entity);
    }

    info!("Setup offscreen Bevy windows");
}

fn release_inactive_debug_window_surface(
    offscreen_windows: Res<AppWindows>,
    mut extracted_windows: ResMut<ExtractedWindows>,
) {
    let Some(debug_window) = offscreen_windows.debug else {
        return;
    };
    let Some(window) = extracted_windows.get_mut(&debug_window) else {
        return;
    };

    if window.swap_chain_texture.is_some() {
        window.present();
        window.needs_initial_present = false;
    }
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

#[wasm_bindgen]
pub fn resize(
    instance_id: String,
    debug_width: u32,
    debug_height: u32,
    map_width: u32,
    map_height: u32,
    scale_factor: f32,
) -> Result<(), String> {
    with_app_world(&instance_id, |world| {
        let app_windows = world.resource::<AppWindows>().clone();
        let (Some(debug), Some(texture)) = (app_windows.debug, app_windows.texture) else {
            return;
        };
        resize_window(world, debug, debug_width, debug_height, scale_factor);
        resize_window(world, texture, map_width, map_height, scale_factor);
    })
}

#[wasm_bindgen]
pub fn forward_focus(instance_id: String, focused: bool) -> Result<(), String> {
    with_debug_window(&instance_id, |world, window| {
        let event = WindowFocused { window, focused };
        world.write_message(event.clone());
        world.write_message(BevyWindowEvent::WindowFocused(event));
    })
}

#[wasm_bindgen]
pub fn forward_cursor_entered(instance_id: String) -> Result<(), String> {
    with_debug_window(&instance_id, |world, window| {
        let event = CursorEntered { window };
        world.write_message(event.clone());
        world.write_message(BevyWindowEvent::CursorEntered(event));
    })
}

#[wasm_bindgen]
pub fn forward_cursor_left(instance_id: String) -> Result<(), String> {
    with_debug_window(&instance_id, |world, window| {
        if let Some(mut window_component) = world.get_mut::<Window>(window) {
            window_component.set_cursor_position(None);
        }

        let event = CursorLeft { window };
        world.write_message(event.clone());
        world.write_message(BevyWindowEvent::CursorLeft(event));
    })
}

#[wasm_bindgen]
pub fn forward_cursor_moved(
    instance_id: String,
    x: f32,
    y: f32,
    delta_x: f32,
    delta_y: f32,
) -> Result<(), String> {
    with_debug_window(&instance_id, |world, window| {
        let position = Vec2::new(x, y);
        let delta = Vec2::new(delta_x, delta_y);

        if let Some(mut window_component) = world.get_mut::<Window>(window) {
            window_component.set_cursor_position(Some(position));
        }

        let event = CursorMoved {
            window,
            position,
            delta: Some(delta),
        };
        world.write_message(event.clone());
        world.write_message(BevyWindowEvent::CursorMoved(event));
        world.write_message(MouseMotion { delta });
        world.write_message(BevyWindowEvent::MouseMotion(MouseMotion { delta }));
    })
}

#[wasm_bindgen]
pub fn forward_mouse_button(instance_id: String, button: i16, pressed: bool) -> Result<(), String> {
    with_debug_window(&instance_id, |world, window| {
        let event = MouseButtonInput {
            button: web_mouse_button(button),
            state: button_state(pressed),
            window,
        };
        world.write_message(event);
        world.write_message(BevyWindowEvent::MouseButtonInput(event));
    })
}

#[wasm_bindgen]
pub fn forward_mouse_wheel(
    instance_id: String,
    delta_x: f32,
    delta_y: f32,
    delta_mode: u32,
) -> Result<(), String> {
    with_debug_window(&instance_id, |world, window| {
        let event = MouseWheel {
            unit: if delta_mode == 1 {
                MouseScrollUnit::Line
            } else {
                MouseScrollUnit::Pixel
            },
            x: delta_x,
            y: -delta_y,
            window,
        };
        world.write_message(event);
        world.write_message(BevyWindowEvent::MouseWheel(event));
    })
}

#[wasm_bindgen]
pub fn forward_keyboard_input(
    instance_id: String,
    code: String,
    key: String,
    pressed: bool,
    repeat: bool,
) -> Result<(), String> {
    with_debug_window(&instance_id, |world, window| {
        let logical_key = web_logical_key(&key);
        let text = match (&logical_key, pressed) {
            (Key::Character(text), true) => Some(text.clone()),
            _ => None,
        };
        let event = KeyboardInput {
            key_code: web_key_code(&code),
            logical_key,
            state: button_state(pressed),
            text,
            repeat,
            window,
        };
        world.write_message(event.clone());
        world.write_message(BevyWindowEvent::KeyboardInput(event));
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
    windows: Res<AppWindows>,
    integrations: Query<(Entity, &MaplibreMapIntegration), Added<MaplibreMapIntegration>>,
) {
    let (Some(debug), Some(texture)) = (windows.debug, windows.texture) else {
        return;
    };
    let app_windows = AppWindows {
        debug: Some(debug),
        texture: Some(texture),
    };
    for (int_entity, _) in integrations.iter() {
        spawn_map_view(&mut commands, int_entity, &app_windows);
    }
}

fn with_app_world(instance_id: &str, f: impl FnOnce(&mut World)) -> Result<(), String> {
    BEVY_APPS.with(|apps| {
        let apps = apps.borrow();
        let Some(managed_app) = apps.get(instance_id) else {
            return Err(format!("Instance {instance_id} is not mounted"));
        };
        let mut managed_app = managed_app.borrow_mut();
        let Some(app) = managed_app.app.as_mut() else {
            return Err(format!("Instance {instance_id} is not mounted"));
        };
        f(app.world_mut());
        Ok(())
    })
}

fn with_debug_window(instance_id: &str, f: impl FnOnce(&mut World, Entity)) -> Result<(), String> {
    with_app_world(instance_id, |world| {
        if let Some(debug) = world.resource::<AppWindows>().debug {
            f(world, debug);
        }
    })
}

fn resize_window(world: &mut World, entity: Entity, width: u32, height: u32, scale_factor: f32) {
    let Some((scale_factor_changed, resized)) =
        world.get_mut::<Window>(entity).map(|mut window| {
            let scale_factor = scale_factor.max(1.0);
            let scale_factor_changed = (window.scale_factor() - scale_factor).abs() > f32::EPSILON;
            let size_changed =
                window.physical_width() != width || window.physical_height() != height;

            if !scale_factor_changed && !size_changed {
                return (false, None);
            }

            window.resolution.set_scale_factor(scale_factor);
            window.resolution.set_physical_resolution(width, height);

            let resized = WindowResized {
                window: entity,
                width: window.width(),
                height: window.height(),
            };
            (scale_factor_changed, Some(resized))
        })
    else {
        return;
    };

    if scale_factor_changed {
        let event = WindowScaleFactorChanged {
            window: entity,
            scale_factor: scale_factor.max(1.0) as f64,
        };
        world.write_message(event.clone());
        world.write_message(BevyWindowEvent::WindowScaleFactorChanged(event));
    }

    if let Some(event) = resized {
        world.write_message(event.clone());
        world.write_message(BevyWindowEvent::WindowResized(event));
    }
}

fn button_state(pressed: bool) -> ButtonState {
    if pressed {
        ButtonState::Pressed
    } else {
        ButtonState::Released
    }
}

fn web_mouse_button(button: i16) -> MouseButton {
    match button {
        0 => MouseButton::Left,
        1 => MouseButton::Middle,
        2 => MouseButton::Right,
        3 => MouseButton::Back,
        4 => MouseButton::Forward,
        other => MouseButton::Other(other as u16),
    }
}

fn web_logical_key(key: &str) -> Key {
    match key {
        "Alt" => Key::Alt,
        "Backspace" => Key::Backspace,
        "Control" => Key::Control,
        "Delete" => Key::Delete,
        "Enter" => Key::Enter,
        "Escape" => Key::Escape,
        "Meta" => Key::Meta,
        "Shift" => Key::Shift,
        "Tab" => Key::Tab,
        "ArrowDown" => Key::ArrowDown,
        "ArrowLeft" => Key::ArrowLeft,
        "ArrowRight" => Key::ArrowRight,
        "ArrowUp" => Key::ArrowUp,
        "F1" => Key::F1,
        "F2" => Key::F2,
        "F3" => Key::F3,
        "F4" => Key::F4,
        "F5" => Key::F5,
        "F6" => Key::F6,
        "F7" => Key::F7,
        "F8" => Key::F8,
        "F9" => Key::F9,
        "F10" => Key::F10,
        "F11" => Key::F11,
        "F12" => Key::F12,
        "" => Key::Unidentified(NativeKey::Unidentified),
        text => Key::Character(text.into()),
    }
}

fn web_key_code(code: &str) -> KeyCode {
    match code {
        "Backspace" => KeyCode::Backspace,
        "Delete" => KeyCode::Delete,
        "Enter" => KeyCode::Enter,
        "Escape" => KeyCode::Escape,
        "F1" => KeyCode::F1,
        "F2" => KeyCode::F2,
        "F3" => KeyCode::F3,
        "F4" => KeyCode::F4,
        "F5" => KeyCode::F5,
        "F6" => KeyCode::F6,
        "F7" => KeyCode::F7,
        "F8" => KeyCode::F8,
        "F9" => KeyCode::F9,
        "F10" => KeyCode::F10,
        "F11" => KeyCode::F11,
        "F12" => KeyCode::F12,
        "KeyA" => KeyCode::KeyA,
        "KeyB" => KeyCode::KeyB,
        "KeyC" => KeyCode::KeyC,
        "KeyD" => KeyCode::KeyD,
        "KeyE" => KeyCode::KeyE,
        "KeyF" => KeyCode::KeyF,
        "KeyG" => KeyCode::KeyG,
        "KeyH" => KeyCode::KeyH,
        "KeyI" => KeyCode::KeyI,
        "KeyJ" => KeyCode::KeyJ,
        "KeyK" => KeyCode::KeyK,
        "KeyL" => KeyCode::KeyL,
        "KeyM" => KeyCode::KeyM,
        "KeyN" => KeyCode::KeyN,
        "KeyO" => KeyCode::KeyO,
        "KeyP" => KeyCode::KeyP,
        "KeyQ" => KeyCode::KeyQ,
        "KeyR" => KeyCode::KeyR,
        "KeyS" => KeyCode::KeyS,
        "KeyT" => KeyCode::KeyT,
        "KeyU" => KeyCode::KeyU,
        "KeyV" => KeyCode::KeyV,
        "KeyW" => KeyCode::KeyW,
        "KeyX" => KeyCode::KeyX,
        "KeyY" => KeyCode::KeyY,
        "KeyZ" => KeyCode::KeyZ,
        "Space" => KeyCode::Space,
        "Tab" => KeyCode::Tab,
        "ArrowDown" => KeyCode::ArrowDown,
        "ArrowLeft" => KeyCode::ArrowLeft,
        "ArrowRight" => KeyCode::ArrowRight,
        "ArrowUp" => KeyCode::ArrowUp,
        "ShiftLeft" => KeyCode::ShiftLeft,
        "ShiftRight" => KeyCode::ShiftRight,
        "ControlLeft" => KeyCode::ControlLeft,
        "ControlRight" => KeyCode::ControlRight,
        "AltLeft" => KeyCode::AltLeft,
        "AltRight" => KeyCode::AltRight,
        "MetaLeft" => KeyCode::SuperLeft,
        "MetaRight" => KeyCode::SuperRight,
        _ => KeyCode::Unidentified(NativeKeyCode::Unidentified),
    }
}
