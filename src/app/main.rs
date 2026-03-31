#[cfg(target_family = "wasm")]
extern crate wasm_bindgen;

use bevy::app::{App, PluginGroup, Startup};
use bevy::asset::Assets;
use bevy::camera::Camera3d;
use bevy::color::Color;
use bevy::DefaultPlugins;
use bevy::light::PointLight;
use bevy::log::{Level, LogPlugin};
use bevy::math::{Quat, Vec3};
use bevy::mesh::{Mesh, Mesh3d};
use bevy::pbr::{MeshMaterial3d, StandardMaterial};
use bevy::prelude::*;
use bevy::render::render_resource::WgpuFeatures;
use bevy::render::RenderPlugin;
use bevy::render::settings::{RenderCreation, WgpuSettings};
use bevy::window::{PresentMode, WindowResolution, WindowTheme};
#[cfg(target_family = "wasm")]
use wasm_bindgen::prelude::*;
use crate::app::js::interop::get_inner_window_dimensions;

#[cfg(target_family = "wasm")]
#[wasm_bindgen]
extern {
    fn alert(s: &str);
}

#[cfg(target_family = "wasm")]
#[wasm_bindgen]
pub fn initialize() {
    let (width, height) = get_inner_window_dimensions().unwrap();

    App::new()
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        resolution: WindowResolution::new(width as u32, height as u32),
                        present_mode: PresentMode::AutoVsync,
                        // do not fit based on DOM location and DOM canvas initial size
                        fit_canvas_to_parent: true,
                        // prevents overriding standard browser input event handling
                        prevent_default_event_handling: false,
                        window_theme: Some(WindowTheme::Light),
                        canvas: Some("#render-canvas".into()),
                        focused: false,
                        ..default()
                    }),
                    ..default()
                })
                .set(LogPlugin {
                    filter: "info,bevy_mod_picking=info,wgpu_core=error,wgpu_hal=error".into(),
                    level: if cfg!(feature = "debug") {
                        Level::INFO
                    } else {
                        Level::WARN
                    },
                    ..default()
                })
                .set(RenderPlugin {
                    render_creation: RenderCreation::Automatic(WgpuSettings {
                        features: if cfg!(target_family = "wasm") {
                            WgpuFeatures::default()
                        } else {
                            WgpuFeatures::default() | WgpuFeatures::POLYGON_MODE_LINE
                        },
                        ..default()
                    }),
                    ..default()
                }),
        )
        .add_systems(Startup, setup)
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // circular base
    commands.spawn((
        Mesh3d(meshes.add(Circle::new(4.0))),
        MeshMaterial3d(materials.add(Color::WHITE)),
        Transform::from_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)),
    ));
    // cube
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(1.0, 1.0, 1.0))),
        MeshMaterial3d(materials.add(Color::srgb_u8(124, 144, 255))),
        Transform::from_xyz(0.0, 0.5, 0.0),
    ));
    // light
    commands.spawn((
        PointLight {
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(4.0, 8.0, 4.0),
    ));
    // camera
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(-2.5, 4.5, 9.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}
