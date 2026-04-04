use crate::app::geo::locations::LocationsManager;
use crate::app::geo::tiling::Tiling;
use crate::app::geo::{GeoMapElementOf, GeoMapPlane, GeoMapPlaneView, GeoMapPlugin};
use crate::app::settings::SettingsPlugin;
use crate::geo::coords::BoundedMercatorProjection;
use bevy::DefaultPlugins;
use bevy::app::{App, PluginGroup, Startup};
use bevy::log::{Level, LogPlugin};
use bevy::prelude::*;
use bevy_pancam::{PanCam, PanCamPlugin};
use bevy_prototype_lyon::plugin::ShapePlugin;
use bevy_vector_shapes::Shape2dPlugin;
use std::f32::consts::PI;

pub fn initialize(_width: usize, _height: usize) {
    App::new()
        .insert_resource(ClearColor(Color::WHITE))
        .add_plugins(DefaultPlugins.set(LogPlugin {
            filter: "info,bevy_mod_picking=info,wgpu_core=error,wgpu_hal=error".into(),
            level: if cfg!(feature = "debug") {
                Level::INFO
            } else {
                Level::WARN
            },
            ..default()
        }))
        .add_plugins((
            GeoMapPlugin {},
            PanCamPlugin,
            ShapePlugin,
            Shape2dPlugin::default(),
            bevy_tokio_tasks::TokioTasksPlugin::default(),
            SettingsPlugin::default(),
        ))
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn((Camera2d, PanCam::default()));

    let mut plane_commands = commands.spawn((
        Transform::default().with_scale(Vec3::ONE),
        Visibility::default(),
        GeoMapPlane {
            projection: BoundedMercatorProjection {
                lat_min: -0.45 * PI,
                lat_max: 0.4 * PI,
            },
            scale: 500.0,
        },
        GeoMapPlaneView::default(),
    ));

    plane_commands.with_child((
        Tiling::new(4),
        Transform::default(),
        Visibility::default(),
        GeoMapElementOf(plane_commands.id()),
    ));

    plane_commands.with_child((
        LocationsManager {},
        Transform::from_translation(Vec3::new(0.0, 0.0, 50.0)),
        Visibility::default(),
        GeoMapElementOf(plane_commands.id()),
    ));
}
