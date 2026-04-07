use crate::app::geo::GeoPlugin;
use crate::app::geo::map::{Map, MapView, MapViewCamera, MapViewCameraWithView, MapViewWithMap};
use crate::app::geo::tiling::{MapViewTiling, MapViewTilingWithView};
use crate::app::settings::SettingsPlugin;
use crate::geo::coords::BoundedMercatorProjection;
use bevy::DefaultPlugins;
use bevy::app::{App, PluginGroup, Startup};
use bevy::log::{Level, LogPlugin};
use bevy::pbr::wireframe::WireframeConfig;
use bevy::prelude::*;
use bevy::render::RenderPlugin;
use bevy::render::settings::{WgpuFeatures, WgpuSettings};
use bevy_inspector_egui::bevy_egui::EguiPlugin;
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use bevy_pancam::{PanCam, PanCamPlugin};
use bevy_prototype_lyon::plugin::ShapePlugin;
use bevy_vector_shapes::Shape2dPlugin;
use std::f64::consts::PI;

pub fn initialize(_width: usize, _height: usize) {
    App::new()
        .insert_resource(ClearColor(Color::WHITE))
        .insert_resource(WireframeConfig {
            global: false,
            default_color: Color::srgb(0.1, 0.1, 1.0),
        })
        .add_plugins(
            DefaultPlugins
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
                    render_creation: WgpuSettings {
                        features: WgpuFeatures::POLYGON_MODE_LINE,
                        ..default()
                    }
                    .into(),
                    ..default()
                }),
        )
        .add_plugins((
            PanCamPlugin,
            GeoPlugin {},
            // MeshPickingPlugin,
            EguiPlugin::default(),
            WorldInspectorPlugin::new(),
            ShapePlugin,
            Shape2dPlugin::default(),
            bevy_tokio_tasks::TokioTasksPlugin::default(),
            SettingsPlugin::default(),
        ))
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands) {
    let map_id = commands
        .spawn(Map {
            projection: BoundedMercatorProjection {
                lat_min: -0.45 * PI,
                lat_max: 0.4 * PI,
            },
        })
        .id();

    let map_view_id = commands
        .spawn((
            Transform::from_scale((5000.0 * Vec2::ONE).extend(1.0)),
            Visibility::default(),
            MapView::default(),
            MapViewWithMap(map_id),
        ))
        .id();

    commands.spawn((MapViewTiling::new(6), MapViewTilingWithView(map_view_id)));

    commands.spawn((
        Camera2d,
        PanCam { ..default() },
        MapViewCamera {},
        MapViewCameraWithView(map_view_id),
    ));

    // let mut plane_commands = commands.spawn((
    //     Transform::default().with_scale(Vec3::ONE),
    //     Visibility::default(),
    //     GeoMapPlane {
    //         projection: BoundedMercatorProjection {
    //             lat_min: -0.45 * PI,
    //             lat_max: 0.4 * PI,
    //         },
    //         scale: 500.0,
    //     },
    //     GeoMapPlaneView::default(),
    // ));
    //
    // plane_commands.with_child((
    //     Tiling::new(4),
    //     Transform::default(),
    //     Visibility::default(),
    //     GeoMapElementOf(plane_commands.id()),
    // ));
    //
    // plane_commands.with_child((
    //     LocationsManager::default(),
    //     Transform::from_translation(Vec3::new(0.0, 0.0, 50.0)),
    //     Visibility::default(),
    //     GeoMapElementOf(plane_commands.id()),
    // ));
}
