use crate::app::maplibre_gl_js::types::{
    MaplibreFeatureData, MaplibreMapViewData, MaplibreTerrainData,
};
use bevy::prelude::*;
use std::cell::Cell;

thread_local! {
    pub(super) static NEXT_INTEGRATION_ID: Cell<u32> = const { Cell::new(1) };
}

#[derive(Component, Default)]
pub struct MaplibreMapIntegration {
    pub id: u32,
    pub view: MaplibreMapViewData,
    pub terrain: MaplibreTerrainData,
    pub features: MaplibreFeatureData,
}

pub(super) fn with_map_data_mut(
    world: &mut World,
    integration_id: u32,
    callback: impl FnOnce(&mut MaplibreMapIntegration),
) {
    let Some(entity) = find_map_integration(world, integration_id) else {
        return;
    };
    let Some(mut map_data) = world.get_mut::<MaplibreMapIntegration>(entity) else {
        return;
    };

    callback(&mut map_data);
}

pub(super) fn find_map_integration(world: &mut World, integration_id: u32) -> Option<Entity> {
    let mut query = world.query::<(Entity, &MaplibreMapIntegration)>();
    query
        .iter(world)
        .find(|(_, integration)| integration.id == integration_id)
        .map(|(entity, _)| entity)
}
