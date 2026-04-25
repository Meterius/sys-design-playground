use crate::app::common::settings::SettingsPlugin;
use crate::app::editor::{EditorPlugin, UiState};
use crate::app::geo::GeoPlugin;
use crate::app::geo::elements_grid::buildings::spawn_building_elements_grid;
use crate::app::geo::elements_grid::roads::spawn_road_elements_grid;
use crate::app::geo::elements_grid::water::spawn_water_elements_grid;
use crate::app::geo::geometry::{MapLine, MapRegion};
use crate::app::geo::map::{
    Map, MapView, MapViewCamera, MapViewCameraWithView, MapViewContextQuery, MapViewWithMap,
};
use crate::app::geo::tiling::manager::{MapViewTiling, MapViewTilingWithView};
use crate::app::utils::big_space_ext::CommandsWithSpatial;
use crate::app::utils::synced_cam::{SyncedCam, SyncedCamPlugin};
use crate::app::utils::vello_ext::VelloExtPlugin;
use crate::geo::coords::{BoundedMercatorProjection, Projection2D};
use bevy::DefaultPlugins;
use bevy::app::{App, PluginGroup, Startup};
use bevy::camera::visibility::RenderLayers;
use bevy::ecs::error::DefaultErrorHandler;
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
use bevy_vello::VelloPlugin;
use bevy_vello::prelude::VelloView;
use big_space::plugin::BigSpaceDefaultPlugins;
use big_space::prelude::{BigSpaceCommands, FloatingOrigin, Grid};
use glam::dvec2;
use itertools::Itertools;
use osm::postgres_integration::client::{OsmClient, OsmClientConfig};
use shapefile::dbase::FieldValue;
use std::env;
use std::f64::consts::PI;
use std::sync::Arc;
use utilities::glam_ext::bounding::AxisAlignedBoundingBox2D;
use crate::app::geo::elements_grid::landuse::spawn_landuse_elements_grid;

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
            // Vello appears to have issues when a large number of elements are drawn in the same render, see https://github.com/linebender/vello/issues/366
            // this causes the render call to not update the texture, making it appear frozen on screen.
            // TODO: implement work-around, e.g. adjusting bevy_vello to draw in multiple stages or draw to a texture instead such that for example each tile or scene can be independent
            VelloPlugin {
                canvas_render_layers: RenderLayers::layer(4),
                ..default()
            },
            VelloExtPlugin,
            SyncedCamPlugin,
        ))
        .add_systems(Startup, setup)
        .add_systems(Update, setup_cam)
        .configure_sets(Update, PanCamSystems.run_if(UiState::allow_game_interaction))
        .insert_resource(DefaultErrorHandler(bevy::ecs::error::error))
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

fn make_osm_config_from_env() -> anyhow::Result<tokio_postgres::Config> {
    let mut config = tokio_postgres::Config::new();
    config
        .user(&env::var("INFRA_GEO_POSTGRES_USER")?)
        .password(&env::var("INFRA_GEO_POSTGRES_PASSWORD")?)
        .host(&env::var("INFRA_GEO_POSTGRES_HOST")?)
        .dbname(&env::var("INFRA_GEO_POSTGRES_DB_NAME")?);
    Ok(config)
}

fn setup(mut commands: Commands, runtime: Res<TokioTasksRuntime>) {
    // TODO: decouple app startup and map view application to allow for independent applications to run without
    // resource/component conflicts

    let map = Map {
        projection: BoundedMercatorProjection {
            lat_min: -0.45 * PI,
            lat_max: 0.4 * PI,
        },
    };

    let map_id = commands.spawn((Name::new("Map"), map.clone())).id();

    let mut cam_id = None;

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

        cam_id = Some(root_grid.spawn_spatial((
            Camera2d,
            Camera {
                order: -1,
                ..default()
            },
            BackgroundColor(Color::WHITE.with_luminance(0.4)),
            PanCam { ..default() },
            MapViewCamera {},
            MapViewCameraWithView(map_view_id),
            FloatingOrigin,
            RenderLayers::from_layers(&[0, 1, 2]),
        )).id());

        runtime.spawn_background_task(async move |mut task| {
            let get_client = async || -> anyhow::Result<OsmClient> {
                let config = make_osm_config_from_env()?;
                Ok(OsmClient::connect(OsmClientConfig { database_config: config }).await?)
            };

            if let Ok(client) = get_client().await.inspect_err(|err| error!("{:?}", err)) {
            task.run_on_main_thread(move |ctx| {
                    let client = Arc::new(client);
                    spawn_road_elements_grid(&mut ctx.world.commands(), map_view_id, client.clone());
                    spawn_building_elements_grid(&mut ctx.world.commands(), map_view_id, client.clone());
                    spawn_water_elements_grid(&mut ctx.world.commands(), map_view_id, client.clone());
                    spawn_landuse_elements_grid(&mut ctx.world.commands(), map_view_id, client.clone());
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

    let cam_id = cam_id.unwrap();

    commands.entity(cam_id).with_child((
        Transform::default(),
        Camera2d,
        VelloView,
        Camera {
            clear_color: ClearColorConfig::Custom(Color::srgba(0., 0., 0., 0.)),
            order: 0,
            ..default()
        },
        // TODO: refactor as unreliable, requires knowledge on camera positioning and does not scale well with additional layers
        SyncedCam {
            main_camera_id: cam_id,
        },
        RenderLayers::layer(4),
    ));
}
