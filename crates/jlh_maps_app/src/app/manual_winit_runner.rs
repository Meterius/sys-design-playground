use crate::app::main::SharedBevyApp;
use bevy::a11y::AccessibilityRequested;
use bevy::ecs::system::SystemState;
use bevy::input::mouse::{MouseButtonInput, MouseMotion, MouseScrollUnit, MouseWheel};
use bevy::math::DVec2;
use bevy::prelude::*;
use bevy::window::{
    CursorEntered, CursorLeft, CursorMoved, Window as BevyWindow, WindowEvent as BevyWindowEvent,
    WindowFocused, WindowResized,
};
use bevy_winit::accessibility::WinitActionRequestHandlers;
use bevy_winit::converters::{convert_element_state, convert_keyboard_input, convert_mouse_button};
use bevy_winit::{
    CreateWindowParams, DisplayHandleWrapper, WINIT_WINDOWS, WinitMonitors, WinitUserEvent,
};
use std::collections::HashMap;
use tracing::trace;
use winit::application::ApplicationHandler;
use winit::event::{MouseScrollDelta, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::WindowId;

#[cfg(target_arch = "wasm32")]
use winit::platform::web::EventLoopExtWebSys;

pub fn prepare_manual_winit_app(app: &mut App, event_loop: &EventLoop<WinitUserEvent>) {
    app.init_resource::<WinitMonitors>()
        .init_resource::<WinitActionRequestHandlers>()
        .init_resource::<AccessibilityRequested>()
        .insert_resource(DisplayHandleWrapper(event_loop.owned_display_handle()));
}

pub fn spawn_manual_winit_runner(
    instance_id: String,
    event_loop: EventLoop<WinitUserEvent>,
    app: SharedBevyApp,
) {
    let runner = ManualWinitRunner {
        instance_id,
        app,
        last_cursor_positions: HashMap::default(),
    };

    trace!("starting manual winit event loop");

    #[cfg(target_arch = "wasm32")]
    event_loop.spawn_app(runner);

    #[cfg(not(target_arch = "wasm32"))]
    {
        let mut runner = runner;
        if let Err(err) = event_loop.run_app(&mut runner) {
            error!("manual winit event loop returned an error: {err}");
        }
    }
}

struct ManualWinitRunner {
    instance_id: String,
    app: SharedBevyApp,
    last_cursor_positions: HashMap<Entity, DVec2>,
}

impl ManualWinitRunner {
    fn create_windows(&mut self, event_loop: &ActiveEventLoop) {
        let mut managed_app = self.app.borrow_mut();
        let Some(app) = managed_app.app.as_mut() else {
            return;
        };

        let mut create_window = SystemState::<CreateWindowParams>::from_world(app.world_mut());
        bevy_winit::create_windows(event_loop, create_window.get_mut(app.world_mut()));
        create_window.apply(app.world_mut());
    }

    fn window_entity(&self, window_id: WindowId) -> Option<Entity> {
        WINIT_WINDOWS.with_borrow(|windows| windows.winit_to_entity.get(&window_id).copied())
    }

    fn with_world(&mut self, f: impl FnOnce(&mut World)) {
        let mut managed_app = self.app.borrow_mut();
        let Some(app) = managed_app.app.as_mut() else {
            return;
        };

        f(app.world_mut());
    }

    fn handle_window_event(&mut self, window_id: WindowId, event: WindowEvent) {
        let Some(window) = self.window_entity(window_id) else {
            return;
        };

        match event {
            WindowEvent::Resized(size) => {
                self.with_world(|world| {
                    if let Some(mut bevy_window) = world.get_mut::<BevyWindow>(window) {
                        bevy_window
                            .resolution
                            .set_physical_resolution(size.width, size.height);

                        let resized = WindowResized {
                            window,
                            width: bevy_window.width(),
                            height: bevy_window.height(),
                        };
                        world.write_message(resized.clone());
                        world.write_message(BevyWindowEvent::WindowResized(resized));
                    }
                });
            }
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                self.with_world(|world| {
                    if let Some(mut bevy_window) = world.get_mut::<BevyWindow>(window) {
                        bevy_window.resolution.set_scale_factor(scale_factor as f32);
                    }
                });
            }
            WindowEvent::Focused(focused) => {
                self.with_world(|world| {
                    let event = WindowFocused { window, focused };
                    world.write_message(event.clone());
                    world.write_message(BevyWindowEvent::WindowFocused(event));
                });
            }
            WindowEvent::CursorEntered { .. } => {
                self.with_world(|world| {
                    let event = CursorEntered { window };
                    world.write_message(event.clone());
                    world.write_message(BevyWindowEvent::CursorEntered(event));
                });
            }
            WindowEvent::CursorLeft { .. } => {
                self.last_cursor_positions.remove(&window);
                self.with_world(|world| {
                    let event = CursorLeft { window };
                    world.write_message(event.clone());
                    world.write_message(BevyWindowEvent::CursorLeft(event));
                });
            }
            WindowEvent::CursorMoved { position, .. } => {
                let physical_position = DVec2::new(position.x, position.y);
                let last_position = self.last_cursor_positions.insert(window, physical_position);
                let physical_delta = last_position.map(|last| physical_position - last);

                self.with_world(|world| {
                    let scale_factor = world
                        .get::<BevyWindow>(window)
                        .map(|window| window.resolution.scale_factor() as f64)
                        .unwrap_or(1.0);
                    let position = (physical_position / scale_factor).as_vec2();
                    let delta = physical_delta.map(|delta| (delta / scale_factor).as_vec2());
                    let cursor_moved = CursorMoved {
                        window,
                        position,
                        delta,
                    };
                    world.write_message(cursor_moved.clone());
                    world.write_message(BevyWindowEvent::CursorMoved(cursor_moved));

                    if let Some(delta) = delta {
                        world.write_message(MouseMotion { delta });
                        world.write_message(BevyWindowEvent::MouseMotion(MouseMotion { delta }));
                    }
                });
            }
            WindowEvent::MouseInput { state, button, .. } => {
                self.with_world(|world| {
                    let event = MouseButtonInput {
                        button: convert_mouse_button(button),
                        state: convert_element_state(state),
                        window,
                    };
                    world.write_message(event);
                    world.write_message(BevyWindowEvent::MouseButtonInput(event));
                });
            }
            WindowEvent::MouseWheel { delta, .. } => {
                self.with_world(|world| {
                    let event = match delta {
                        MouseScrollDelta::LineDelta(x, y) => MouseWheel {
                            unit: MouseScrollUnit::Line,
                            x,
                            y,
                            window,
                        },
                        MouseScrollDelta::PixelDelta(position) => MouseWheel {
                            unit: MouseScrollUnit::Pixel,
                            x: position.x as f32,
                            y: position.y as f32,
                            window,
                        },
                    };
                    world.write_message(event);
                    world.write_message(BevyWindowEvent::MouseWheel(event));
                });
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if event.repeat {
                    return;
                }
                self.with_world(|world| {
                    let event = convert_keyboard_input(&event, window);
                    world.write_message(event.clone());
                    world.write_message(BevyWindowEvent::KeyboardInput(event));
                });
            }
            _ => {}
        }
    }
}

impl ApplicationHandler<WinitUserEvent> for ManualWinitRunner {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        trace!("manual winit runner resumed for {}", self.instance_id);
        self.create_windows(event_loop);
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: WinitUserEvent) {
        if matches!(event, WinitUserEvent::WindowAdded) {
            self.create_windows(event_loop);
        }
    }

    fn window_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        self.handle_window_event(window_id, event);
    }
}
