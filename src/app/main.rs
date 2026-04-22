use crate::app::common::settings::SettingsPlugin;
use crate::app::editor::{EditorPlugin, UiState};
use crate::app::geo::GeoPlugin;
use crate::app::geo::elements_grid::spawn_road_elements_grid;
use crate::app::geo::geometry::{MapLine, MapRegion};
use crate::app::geo::map::{
    Map, MapView, MapViewCamera, MapViewCameraWithView, MapViewContextQuery, MapViewWithMap,
};
use crate::app::geo::tiling::manager::{MapViewTiling, MapViewTilingWithView};
use crate::app::utils::big_space_ext::CommandsWithSpatial;
use crate::geo::coords::{BoundedMercatorProjection, Projection2D};
use crate::geo::osm::client::OsmClient;
use bevy::DefaultPlugins;
use bevy::app::{App, PluginGroup, Startup};
use bevy::camera::visibility::RenderLayers;
use bevy::log::{Level, LogPlugin};
use bevy::pbr::wireframe::WireframeConfig;
use bevy::prelude::*;
use bevy::render::RenderPlugin;
use bevy::render::settings::{WgpuFeatures, WgpuSettings};
use bevy::sprite_render::Wireframe2dConfig;
use bevy_inspector_egui::bevy_egui::EguiPlugin;
use bevy_pancam::{PanCam, PanCamPlugin, PanCamSystems};
use bevy_prototype_lyon::plugin::ShapePlugin;
use bevy_tokio_tasks::TokioTasksRuntime;
use big_space::plugin::BigSpaceDefaultPlugins;
use big_space::prelude::{BigSpaceCommands, FloatingOrigin, Grid};
use glam::dvec2;
use itertools::Itertools;
use shapefile::dbase::FieldValue;
use std::f64::consts::PI;
use std::sync::Arc;
use utilities::glam_ext::bounding::AxisAlignedBoundingBox2D;

pub fn initialize(_width: usize, _height: usize) {
    App::new()
        .insert_resource(ClearColor(Color::WHITE))
        .insert_resource(Wireframe2dConfig {
            global: false,
            default_color: Color::srgb(0.1, 0.1, 1.0),
        })
        .insert_resource(WireframeConfig {
            global: false,
            default_color: Color::srgb(0.1, 0.1, 1.0),
        })
        .add_plugins(
            DefaultPlugins
                .set(LogPlugin {
                    filter: "info,jlh_sys_design_playground=info,bevy_mod_picking=info,wgpu_core=error,wgpu_hal=error".into(),
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
            EguiPlugin::default(),
            PanCamPlugin,
            GeoPlugin {},
            MeshPickingPlugin,
            ShapePlugin,
            bevy_tokio_tasks::TokioTasksPlugin::default(),
            SettingsPlugin::default(),
            BigSpaceDefaultPlugins,
            EditorPlugin {},
        ))
        .add_systems(Startup, setup)
        .add_systems(Update, setup_cam)
        .configure_sets(Update, PanCamSystems.run_if(UiState::allow_game_interaction))
        .run();
}

fn setup_cam(
    cams: Query<(Entity, &mut Transform), Added<MapViewCamera>>,
    view_ctx: MapViewContextQuery,
) {
    for (cam_id, mut cam_tr) in cams {
        if let Some(ctx) = view_ctx.get(cam_id) {
            cam_tr.translation = ctx
                .view
                .abs_to_local(
                    ctx.map
                        .projection
                        .gcs_to_abs(dvec2(9.9872f64.to_radians(), 53.5488f64.to_radians())),
                )
                .as_vec2()
                .extend(0.0);
        }
    }
}

fn setup(mut commands: Commands, runtime: Res<TokioTasksRuntime>) {
    let map = Map {
        projection: BoundedMercatorProjection {
            lat_min: -0.45 * PI,
            lat_max: 0.4 * PI,
        },
    };

    let map_id = commands.spawn((Name::new("Map"), map.clone())).id();

    commands.spawn_big_space(Grid::default(), |root_grid| {
        let map_view = MapView::new();

        let map_view_id = root_grid
            .insert((
                Name::new("Map View"),
                Visibility::default(),
                map_view.clone(),
                MapViewWithMap(map_id),
            ))
            .id();

        // root_grid.spawn_spatial((Grid::default(), Name::new("Tiling"), MapViewTiling::new(6), MapViewTilingWithView(map_view_id)));

        root_grid.spawn_spatial((
            Camera2d,
            BackgroundColor(Color::WHITE.with_luminance(0.4)),
            PanCam { ..default() },
            MapViewCamera {},
            MapViewCameraWithView(map_view_id),
            FloatingOrigin,
            RenderLayers::from_layers(&[0, 1, 2]),
        ));

        runtime.spawn_background_task(async move |mut task| {
            if let Ok(client) = OsmClient::connect().await.inspect_err(|err| error!("{:?}", err)) {
            task.run_on_main_thread(move |ctx| {
                       spawn_road_elements_grid(&mut ctx.world.commands(), map_view_id, Arc::new(client));
                }).await;
            }
        });

        let background_id = root_grid.spawn_spatial((Name::new("Background"), Grid::default())).id();

        runtime.spawn_background_task(async move |mut task| {
            let layers = [
                ("Land", -5.0, None, shapefile::Reader::from_path("./assets/datasets/natural_earth_vector/10m_physical/ne_10m_land.shp").unwrap(), Color::hsv(38.0, 0.12, 0.45)),
                ("Lake", -4.0, None, shapefile::Reader::from_path("./assets/datasets/natural_earth_vector/10m_physical/ne_10m_lakes.shp").unwrap(), Color::hsv(206.0, 0.27, 0.87)),
                ("River", -3.0, None, shapefile::Reader::from_path("./assets/datasets/natural_earth_vector/10m_physical/ne_10m_rivers_lake_centerlines.shp").unwrap(), Color::hsv(206.0, 0.27, 0.87)),
                ("Boundary", 102.0, Some(100.0), shapefile::Reader::from_path("./assets/datasets/natural_earth_vector/10m_cultural/ne_10m_admin_0_boundary_lines_land.shp").unwrap(), Color::hsv(38.0, 0.22, 0.47)),
                //("Railroad", 103.0, Some(15.0), shapefile::Reader::from_path("./assets/datasets/natural_earth_vector/10m_cultural/ne_10m_railroads.shp").unwrap(), Color::hsv(38.0, 0.0, 0.7)),
                //("Road", 104.0, Some(5.0), shapefile::Reader::from_path("./assets/datasets/natural_earth_vector/10m_cultural/ne_10m_admin_0_boundary_lines_land.shp").unwrap(), Color::hsv(38.0, 0.0, 0.5)),
            ];

            // let index = fetch_fabrik_index(&reqwest::Client::new()).await.unwrap();
            task.run_on_main_thread(move |world| {
                let ocean = world
                    .world
                    .commands()
                    .spawn_spatial((
                        Transform::from_translation(vec3(0.0, 0.0, -10.0)),
                        Name::new("Geometry"),
                        MapRegion::new(
                            map.projection.gcs_bounds().corners().collect_vec(),
                            Color::hsv(206.0, 0.27, 0.87),
                        ),
                        RenderLayers::layer(2),
                    ))
                    .id();

                world.world.commands().entity(background_id).add_child(ocean);

                for (label, depth, width, mut shapes, color) in layers.into_iter() {
                    for (shape, rec) in shapes.read().unwrap() {
                        match shape {
                            shapefile::Shape::Polygon(poly) => {
                                for ring in poly.rings() {
                                    let per_id = world
                                        .world
                                        .commands()
                                        .spawn_spatial((
                                            Transform::from_translation(vec3(0.0, 0.0, depth)),
                                            Name::new(label),
                                            MapRegion::new(
                                                ring
                                                    .points()
                                                    .iter()
                                                    .map(|pos| dvec2(pos.x.to_radians(), pos.y.to_radians()))
                                                    .collect_vec(),
                                                color,
                                            ),
                                            RenderLayers::layer(2),
                                        ))
                                        .id();

                                    world.world.commands().entity(background_id).add_child(per_id);
                                }
                            }
                            shapefile::Shape::Polyline(poly) => {
                                let scale = if let Some(FieldValue::Numeric(scale)) = rec.get("scalerank") { *scale } else { None };

                                for part in poly.parts() {
                                    let per_id = world
                                        .world
                                        .commands()
                                        .spawn_spatial((
                                            Transform::from_translation(vec3(0.0, 0.0, depth)),
                                            Name::new(label),
                                            MapLine::new(
                                                part
                                                    .iter()
                                                    .map(|pos| dvec2(pos.x.to_radians(), pos.y.to_radians()))
                                                    .collect_vec(),
                                                width.unwrap_or(if let Some(scale) = scale { (scale.max(1.0) as f32).powf(1.5) * 2.0 } else { 10.0 }),
                                                color,
                                            ),
                                            RenderLayers::layer(2),
                                        ))
                                        .id();

                                    world.world.commands().entity(background_id).add_child(per_id);
                                }
                            }
                            _ => { error!("Unexpected shape") }
                        }
                    }
                }
            })
                .await;
        });
    });
}
