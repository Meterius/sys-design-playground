use bevy::log::LogPlugin;
use bevy::prelude::*;
use bevy::render::{
    settings::{Backends, WgpuSettings, WgpuSettingsPriority},
    RenderPlugin,
};
use bevy::window::{ExitCondition, Window, WindowResolution};
use bevy_winit::{EventLoopProxy, EventLoopProxyWrapper, WinitUserEvent};
use std::cell::{Cell, RefCell};
use std::collections::{HashMap, VecDeque};
use bevy::render::settings::{RenderCreation, WgpuFeatures};
use wasm_bindgen::prelude::wasm_bindgen;

thread_local! {
    static RUNTIME_STARTED: Cell<bool> = const { Cell::new(false) };
    static EVENT_LOOP_PROXY: RefCell<Option<EventLoopProxy<WinitUserEvent>>> = const { RefCell::new(None) };
    static PENDING_COMMANDS: RefCell<VecDeque<MapViewCommand>> = const { RefCell::new(VecDeque::new()) };
}

enum MapViewCommand {
    Mount { canvas_selector: String },
    Unmount { canvas_selector: String },
}

#[derive(Resource, Default)]
struct MapViewWindows {
    by_canvas_selector: HashMap<String, Entity>,
}

#[wasm_bindgen]
pub fn initialize(canvas_selector: String) {
    console_error_panic_hook::set_once();

    if RUNTIME_STARTED.with(|started| started.replace(true)) {
        enqueue_command(MapViewCommand::Mount { canvas_selector });
        wake_event_loop();
        return;
    }

    let mut app = App::new();

    app.add_plugins(
        DefaultPlugins
            .set(WindowPlugin {
                primary_window: Some(map_view_window(canvas_selector)),
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
            }),
    );

    app.init_resource::<MapViewWindows>()
        .add_systems(
            Update,
            (
                cache_event_loop_proxy,
                register_map_view_windows,
                drain_map_view_commands,
            ),
        );

    app.run();
}

#[wasm_bindgen]
pub fn unmount(canvas_selector: String) {
    enqueue_command(MapViewCommand::Unmount { canvas_selector });
    wake_event_loop();
}

fn enqueue_command(command: MapViewCommand) {
    PENDING_COMMANDS.with_borrow_mut(|commands| commands.push_back(command));
}

fn wake_event_loop() {
    EVENT_LOOP_PROXY.with_borrow(|proxy| {
        if let Some(proxy) = proxy {
            let _ = proxy.send_event(WinitUserEvent::WakeUp);
        }
    });
}

fn cache_event_loop_proxy(proxy: Res<EventLoopProxyWrapper>) {
    EVENT_LOOP_PROXY.with_borrow_mut(|stored_proxy| {
        if stored_proxy.is_none() {
            *stored_proxy = Some(proxy.clone());
        }
    });
}

fn register_map_view_windows(
    mut windows: ResMut<MapViewWindows>,
    query: Query<(Entity, &Window), Added<Window>>,
) {
    for (entity, window) in &query {
        if let Some(canvas_selector) = &window.canvas {
            windows
                .by_canvas_selector
                .insert(canvas_selector.clone(), entity);
        }
    }
}

fn drain_map_view_commands(mut commands: Commands, mut windows: ResMut<MapViewWindows>) {
    let queued_commands =
        PENDING_COMMANDS.with_borrow_mut(|pending| pending.drain(..).collect::<Vec<_>>());

    for command in queued_commands {
        match command {
            MapViewCommand::Mount { canvas_selector } => {
                if windows.by_canvas_selector.contains_key(&canvas_selector) {
                    continue;
                }

                let entity = commands
                    .spawn(map_view_window(canvas_selector.clone()))
                    .id();

                windows.by_canvas_selector.insert(canvas_selector, entity);
            }
            MapViewCommand::Unmount { canvas_selector } => {
                if let Some(entity) = windows.by_canvas_selector.remove(&canvas_selector) {
                    commands.entity(entity).despawn();
                }
            }
        }
    }
}

fn map_view_window(canvas_selector: String) -> Window {
    Window {
        canvas: Some(canvas_selector),
        fit_canvas_to_parent: true,
        prevent_default_event_handling: false,
        resolution: WindowResolution::default().with_scale_factor_override(1.0),
        ..default()
    }
}
