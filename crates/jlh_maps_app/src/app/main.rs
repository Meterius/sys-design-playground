use bevy::camera::visibility::RenderLayers;
use bevy::log::LogPlugin;
use bevy::prelude::*;
use bevy::render::{
    RenderPlugin,
    settings::{RenderCreation, WgpuFeatures, WgpuSettings},
};
use bevy::window::{
    ExitCondition, PresentMode, PrimaryWindow, Window, WindowPlugin, WindowResolution,
};
use bevy_inspector_egui::bevy_egui::EguiPlugin;
use std::cell::{Cell, RefCell};
use std::collections::VecDeque;
use wasm_bindgen::prelude::wasm_bindgen;

use crate::app::common::settings::SettingsPlugin;
use crate::app::editor::EditorPlugin;
use crate::app::map::MapViewPlugin;
use crate::app::map::core::{
    MapView, MapViewCamera, MapViewTileManager, spawn_map_view_camera, spawn_map_view_tile_manager,
};
use crate::app::map::integration::MapViewIntegrationId;

thread_local! {
    static RUNTIME_STARTED: Cell<bool> = const { Cell::new(false) };
    static PENDING_RUNTIME_COMMANDS: RefCell<VecDeque<MapViewRuntimeCommand>> = const { RefCell::new(VecDeque::new()) };
}

#[derive(Resource, Default)]
struct MapViewRuntime {
    next_render_layer: usize,
    initial_canvas_selector: Option<String>,
}

#[derive(Debug)]
enum MapViewRuntimeCommand {
    Mount { canvas_selector: String },
    Unmount { canvas_selector: String },
}

#[wasm_bindgen]
pub fn initialize() {
    console_error_panic_hook::set_once();
}

#[wasm_bindgen]
pub fn mount(canvas_selector: String) {
    if runtime_started() {
        enqueue_runtime_command(MapViewRuntimeCommand::Mount { canvas_selector });
        return;
    }

    set_runtime_started();

    let mut app = App::new();

    app.insert_resource(MapViewRuntime {
        next_render_layer: 0,
        initial_canvas_selector: Some(canvas_selector.clone()),
    });

    app.add_plugins((
        DefaultPlugins
            .set(WindowPlugin {
                primary_window: Some(map_view_window(canvas_selector.clone())),
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
        EguiPlugin::default(),
        SettingsPlugin {},
        EditorPlugin {},
        MapViewPlugin,
    ));

    app.add_systems(Startup, mount_initial_map_view)
        .add_systems(PreUpdate, drain_map_view_runtime_commands);

    app.run();
}

#[wasm_bindgen]
pub fn unmount(canvas_selector: String) {
    enqueue_runtime_command(MapViewRuntimeCommand::Unmount { canvas_selector });
}

fn runtime_started() -> bool {
    RUNTIME_STARTED.with(|started| started.get())
}

fn set_runtime_started() {
    RUNTIME_STARTED.with(|started| started.set(true));
}

fn enqueue_runtime_command(command: MapViewRuntimeCommand) {
    PENDING_RUNTIME_COMMANDS.with_borrow_mut(|commands| commands.push_back(command));
}

fn mount_initial_map_view(
    mut commands: Commands,
    mut runtime: ResMut<MapViewRuntime>,
    primary_window: Single<Entity, With<PrimaryWindow>>,
) {
    let Some(canvas_selector) = runtime.initial_canvas_selector.take() else {
        return;
    };

    configure_map_view(
        &mut commands,
        &mut runtime,
        *primary_window,
        canvas_selector,
    );
}

fn drain_map_view_runtime_commands(
    mut commands: Commands,
    mut runtime: ResMut<MapViewRuntime>,
    map_views: Query<(Entity, &MapViewIntegrationId), With<MapView>>,
    cameras: Query<(Entity, &MapViewCamera)>,
    tile_managers: Query<(Entity, &MapViewTileManager)>,
) {
    let queued_commands =
        PENDING_RUNTIME_COMMANDS.with_borrow_mut(|pending| pending.drain(..).collect::<Vec<_>>());

    for command in queued_commands {
        match command {
            MapViewRuntimeCommand::Mount { canvas_selector } => {
                if find_map_view(&map_views, &canvas_selector).is_some() {
                    continue;
                }

                spawn_map_view(&mut commands, &mut runtime, canvas_selector);
            }
            MapViewRuntimeCommand::Unmount { canvas_selector } => {
                let Some(map_view) = find_map_view(&map_views, &canvas_selector) else {
                    continue;
                };

                despawn_map_view(&mut commands, map_view, &cameras, &tile_managers);
            }
        }
    }
}

fn spawn_map_view(
    commands: &mut Commands,
    runtime: &mut MapViewRuntime,
    canvas_selector: String,
) -> Entity {
    let map_view = commands
        .spawn(map_view_window(canvas_selector.clone()))
        .id();
    configure_map_view(commands, runtime, map_view, canvas_selector);

    map_view
}

fn configure_map_view(
    commands: &mut Commands,
    runtime: &mut MapViewRuntime,
    map_view: Entity,
    canvas_selector: String,
) {
    let render_layer = runtime.next_render_layer % 32;
    runtime.next_render_layer += 1;

    commands.entity(map_view).insert((
        MapView { render_layer },
        MapViewIntegrationId { canvas_selector },
        RenderLayers::layer(render_layer),
    ));

    spawn_map_view_camera(commands, map_view, render_layer);
    spawn_map_view_tile_manager(commands, map_view);
}

fn despawn_map_view(
    commands: &mut Commands,
    map_view: Entity,
    cameras: &Query<(Entity, &MapViewCamera)>,
    tile_managers: &Query<(Entity, &MapViewTileManager)>,
) {
    for (camera_entity, camera) in cameras {
        if camera.map_view == Some(map_view) {
            commands.entity(camera_entity).despawn();
        }
    }

    for (tile_manager_entity, tile_manager) in tile_managers {
        if tile_manager.map_view != Some(map_view) {
            continue;
        }

        for tile in tile_manager.tiles.values() {
            commands.entity(tile.entity).despawn();
        }
        commands.entity(tile_manager_entity).despawn();
    }

    commands.entity(map_view).despawn();
}

fn find_map_view(
    query: &Query<(Entity, &MapViewIntegrationId), With<MapView>>,
    canvas_selector: &str,
) -> Option<Entity> {
    query
        .iter()
        .find(|(_, integration_id)| integration_id.canvas_selector == canvas_selector)
        .map(|(entity, _)| entity)
}

fn map_view_window(canvas_selector: String) -> Window {
    Window {
        present_mode: PresentMode::AutoVsync,
        canvas: Some(canvas_selector),
        fit_canvas_to_parent: true,
        prevent_default_event_handling: false,
        resolution: WindowResolution::default().with_scale_factor_override(1.0),
        ..default()
    }
}
