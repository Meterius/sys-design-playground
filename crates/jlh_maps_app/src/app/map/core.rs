use crate::app::map::camera::MapViewCamera;
use bevy::prelude::*;
use big_space::bundles::BigSpaceRootBundle;
use big_space::prelude::CellCoord;

pub(super) struct CorePlugin;

impl Plugin for CorePlugin {
    fn build(&self, _app: &mut App) {}
}

#[derive(Debug, Reflect, Component)]
pub struct MapView {
    pub maplibre_int_id: Entity,
}

pub fn spawn_map_view(commands: &mut Commands, maplibre_integration_id: Entity) {
    let map_view_id = commands
        .spawn((
            Name::new("Map View"),
            BigSpaceRootBundle::default(),
            MapView {
                maplibre_int_id: maplibre_integration_id,
            },
        ))
        .id();

    commands.entity(map_view_id).with_child((
        Transform::default(),
        CellCoord::default(),
        Camera3d::default(),
        MapViewCamera {
            maplibre_int_id: maplibre_integration_id,
        },
    ));
}
