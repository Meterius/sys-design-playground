use crate::app::geo::{GeoMapPlane, GeoMapPlugin, MercatorProjection};
use bevy::DefaultPlugins;
use bevy::app::{App, PluginGroup, Startup};
use bevy::log::{Level, LogPlugin};
use bevy::prelude::*;
use bevy_pancam::{PanCam, PanCamPlugin};
use bevy_prototype_lyon::plugin::ShapePlugin;
use bevy_vector_shapes::Shape2dPlugin;

pub fn initialize(width: usize, height: usize) {
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
        ))
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn((Camera2d, PanCam::default()));

    commands.spawn((
        Transform::default().with_scale(Vec3::ONE),
        Visibility::default(),
        GeoMapPlane {
            projection: MercatorProjection { scale: 5000.0 },
        },
    ));
}
