use std::sync::Arc;
use bevy::input::ButtonState;
use bevy::input::keyboard::KeyboardInput;
use crate::geo::locations::{Location, LocationClient};
use bevy::prelude::*;
use bevy_tokio_tasks::TokioTasksRuntime;
use crate::app::geo::{GeoMapElementOf, GeoMapPlane, GeoMapPlaneView, GeoMapTransform};
use crate::geo::coords::{LonLatVec2, Projection2D, RadLonLatVec2};

#[derive(Default)]
pub struct LocationsPlugin {}

impl Plugin for LocationsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (initialize_locations_manager, request_markers, sync_location_marker));
    }
}

#[derive(Component, Default)]
pub struct LocationsManager {
    client: Option<Arc<LocationClient>>,
}

#[derive(Component)]
pub struct LocationMarker {
    pub location: Location,
}

fn sync_location_marker(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    asset_server: Res<AssetServer>,
    changed_locations: Query<(&LocationMarker, &GeoMapElementOf), Added<LocationMarker>>,
    planes: Query<&GeoMapPlane>,
) {
    for (loc_marker, loc_marker_element_of) in changed_locations.iter() {
        if let Ok(plane) = planes.get(loc_marker_element_of.0) {
            let mut e_commands = commands.spawn((
                Transform::from_translation(
                    plane.abs_to_local(&plane.projection.gcs_to_abs(&RadLonLatVec2::from(loc_marker.location.pos.clone()))).extend(1000.0),
                ).with_scale(Vec3::ONE * 0.5f32.powf(23.0f32) *  plane.scale),
                Visibility::default(),
                GeoMapTransform { pos: RadLonLatVec2::from(loc_marker.location.pos.clone()) },
            ));


            e_commands.with_child((
                Transform::from_xyz(0.0, 0.0, 0.0),
                Mesh2d(meshes.add(Circle::new(5.0))),
                MeshMaterial2d(materials.add(Color::BLACK)),
            ));

            e_commands.with_child((
                Transform::from_xyz(0.0, 15.0, 1.0),
                Text2d(loc_marker.location.name.clone()),
                TextFont {
                    font: asset_server.load("fonts/calibri-regular.ttf"),
                    font_size: 10.0,
                    ..default()
                },
                TextColor(Color::srgb(0.0, 0.0, 0.0)),
                TextBackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.3)),
            ));

            let entity_id = e_commands.id();
            commands.entity(loc_marker_element_of.0).add_child(entity_id);
        }
    }
}

fn request_markers(
    runtime: Res<TokioTasksRuntime>,
    managers: Query<(Entity, &LocationsManager, &GeoMapElementOf)>,
    plane_views: Query<&GeoMapPlaneView>,
    mut keyboard_messages: MessageReader<KeyboardInput>,
) {
    for key in keyboard_messages.read() {
        if let (KeyCode::KeyL, ButtonState::Released) = (key.key_code, key.state) {
            for (manager_id, manager, manager_element_of) in managers.iter() {
                if let Ok((plane_view)) = plane_views.get(manager_element_of.0.clone())
                    && let Some(view_gcs) = plane_view.view_gcs.clone()
                    && let Some(client) = manager.client.as_ref() {
                    let plane_id = manager_element_of.0;
                    let client = client.clone();
                    let view_center_gcs = RadLonLatVec2::from((Vec2::from(view_gcs.0) + Vec2::from(view_gcs.1)) / 2.0);

                    runtime.spawn_background_task(async move |mut task_ctx| {
                        let view_center_gcs = LonLatVec2::from(view_center_gcs.clone());
                        debug!("Requesting {:?}", view_center_gcs);
                        if let Ok(locations) = client.fetch_nearest(
                            view_center_gcs, 100,
                        ).await.inspect_err(|err| { error!("{err}"); }) {
                            debug!("Found {:?}", locations);
                            task_ctx.run_on_main_thread(move |ctx| {
                                for location in locations.into_iter() {
                                    ctx.world.commands().entity(manager_id)
                                        .with_child((
                                            GeoMapElementOf(plane_id),
                                            LocationMarker { location },
                                        ));
                                }
                            }).await;
                        }
                    });
                }
            }
        }
    }
}

fn initialize_locations_manager(
    runtime: Res<TokioTasksRuntime>,
    added_managers: Query<Entity, Added<LocationsManager>>,
) {
    for added_manager in added_managers.iter() {
        runtime.spawn_background_task(async move |mut task_ctx| {
            if let Ok(client) = LocationClient::connect().await.inspect_err(|err| {
                error!("{err}");
            }) {
                task_ctx
                    .run_on_main_thread(move |ctx| {
                        if let Some(mut locations_manager) = ctx
                            .world
                            .entity_mut(added_manager)
                            .get_mut::<LocationsManager>()
                        {
                            debug!("Locations manager connected to database");
                            locations_manager.client = Some(Arc::new(client));
                        }
                    })
                    .await;
            }
        });
    }
}
