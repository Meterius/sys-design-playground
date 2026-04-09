use crate::app::common::settings::SettingsPlugin;
use crate::app::geo::GeoPlugin;
use crate::app::geo::map::{Map, MapView, MapViewCamera, MapViewCameraWithView, MapViewWithMap};
use crate::app::geo::tiling::{MapViewTiling, MapViewTilingWithView};
use crate::geo::coords::BoundedMercatorProjection;
use crate::utils::glam_ext::bounding::AxisAlignedBoundingBox2D;
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
use bevy_tokio_tasks::TokioTasksRuntime;
use bevy_vector_shapes::Shape2dPlugin;
use big_space::plugin::BigSpaceDefaultPlugins;
use big_space::prelude::{BigSpaceCommands, FloatingOrigin, Grid};
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
                })
                .disable::<TransformPlugin>(),
        )
        .add_plugins((
            PanCamPlugin,
            GeoPlugin {},
            MeshPickingPlugin,
            EguiPlugin::default(),
            WorldInspectorPlugin::new(),
            ShapePlugin,
            Shape2dPlugin::default(),
            bevy_tokio_tasks::TokioTasksPlugin::default(),
            SettingsPlugin::default(),
            BigSpaceDefaultPlugins,
        ))
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands, runtime: Res<TokioTasksRuntime>) {
    let map = Map {
        projection: BoundedMercatorProjection {
            lat_min: -0.45 * PI,
            lat_max: 0.4 * PI,
        },
    };

    let map_id = commands.spawn(map.clone()).id();

    commands.spawn_big_space(Grid::default(), |root_grid| {
        let map_view = MapView::new();

        let map_view_id = root_grid
            .insert((
                Visibility::default(),
                map_view.clone(),
                MapViewWithMap(map_id),
            ))
            .id();

        root_grid.spawn_spatial((MapViewTiling::new(6), MapViewTilingWithView(map_view_id)));

        root_grid.spawn_spatial((
            Camera2d,
            BackgroundColor(Color::WHITE.with_luminance(0.4)),
            PanCam { ..default() },
            MapViewCamera {},
            MapViewCameraWithView(map_view_id),
            FloatingOrigin,
        ));
    });

    // runtime.spawn_background_task(async move |mut task| {
    //     let index = fetch_fabrik_index(&reqwest::Client::new()).await.unwrap();
    //     task.run_on_main_thread(move |world| {
    //         for geometry in index
    //             .features
    //             .into_iter()
    //             .filter_map(|feature| feature.geometry)
    //         {
    //             world.world.commands().entity(map_view_id).with_child((
    //                 Transform::from_translation(vec3(0.0, 0.0, 100.0)),
    //                 Visibility::default(),
    //                 Geometry {
    //                     geometry: geometry.value,
    //                 },
    //             ));
    //         }
    //     })
    //         .await;
    // });
}
