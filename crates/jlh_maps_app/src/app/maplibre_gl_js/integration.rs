use crate::app::instance_management::commands::enqueue_instance_command;
use crate::app::maplibre_gl_js::types::{
    CanonicalTileId, MaplibreFeatureData, MaplibreMapViewData, MaplibreTerrainData,
    MaplibreTerrainTileData, SourceLayerFeature,
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

pub(super) enum MaplibreIntegrationCommand {
    CreateMapIntegration {
        instance_id: String,
        integration_id: u32,
    },
    RemoveMapIntegration {
        instance_id: String,
        integration_id: u32,
    },
    SyncView {
        instance_id: String,
        integration_id: u32,
        view: MaplibreMapViewData,
    },
    UpdateTerrainTileData {
        instance_id: String,
        integration_id: u32,
        tile_key: CanonicalTileId,
        tile_data: MaplibreTerrainTileData,
    },
    RemoveTerrainTileData {
        instance_id: String,
        integration_id: u32,
        tile_key: CanonicalTileId,
    },
    UpdateFeatures {
        instance_id: String,
        integration_id: u32,
        features: Vec<SourceLayerFeature>,
    },
    RemoveFeatures {
        instance_id: String,
        integration_id: u32,
        feature_keys: Vec<String>,
    },
}

impl MaplibreIntegrationCommand {
    fn instance_id(&self) -> &str {
        match self {
            MaplibreIntegrationCommand::CreateMapIntegration { instance_id, .. }
            | MaplibreIntegrationCommand::RemoveMapIntegration { instance_id, .. }
            | MaplibreIntegrationCommand::SyncView { instance_id, .. }
            | MaplibreIntegrationCommand::UpdateTerrainTileData { instance_id, .. }
            | MaplibreIntegrationCommand::RemoveTerrainTileData { instance_id, .. }
            | MaplibreIntegrationCommand::UpdateFeatures { instance_id, .. }
            | MaplibreIntegrationCommand::RemoveFeatures { instance_id, .. } => instance_id,
        }
    }
}

pub(super) fn enqueue_command(command: MaplibreIntegrationCommand) -> Result<(), String> {
    let instance_id = command.instance_id().to_owned();
    enqueue_instance_command(&instance_id, move |world| match command {
        MaplibreIntegrationCommand::CreateMapIntegration { integration_id, .. } => {
            world.spawn((
                MaplibreMapIntegration {
                    id: integration_id,
                    ..default()
                },
                Name::new(format!("MapLibre map integration {integration_id}")),
            ));
        }
        MaplibreIntegrationCommand::RemoveMapIntegration { integration_id, .. } => {
            if let Some(entity) = find_map_integration(world, integration_id) {
                world.despawn(entity);
            }
        }
        MaplibreIntegrationCommand::SyncView {
            integration_id,
            view,
            ..
        } => {
            with_map_data_mut(world, integration_id, |map_data| {
                map_data.view = view;
            });
        }
        MaplibreIntegrationCommand::UpdateTerrainTileData {
            integration_id,
            tile_key,
            tile_data,
            ..
        } => {
            with_map_data_mut(world, integration_id, |map_data| {
                map_data.terrain.tiles.insert(tile_key, tile_data);
            });
        }
        MaplibreIntegrationCommand::RemoveTerrainTileData {
            integration_id,
            tile_key,
            ..
        } => {
            with_map_data_mut(world, integration_id, |map_data| {
                map_data.terrain.tiles.remove(&tile_key);
            });
        }
        MaplibreIntegrationCommand::UpdateFeatures {
            integration_id,
            features,
            ..
        } => {
            with_map_data_mut(world, integration_id, |map_data| {
                for feature in features {
                    map_data
                        .features
                        .features
                        .insert(feature.id.clone(), feature);
                }
            });
        }
        MaplibreIntegrationCommand::RemoveFeatures {
            integration_id,
            feature_keys,
            ..
        } => {
            with_map_data_mut(world, integration_id, |map_data| {
                for feature_key in feature_keys {
                    map_data.features.features.remove(&feature_key);
                }
            });
        }
    })
    .map_err(|err| err.to_string())
}

fn with_map_data_mut(
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

fn find_map_integration(world: &mut World, integration_id: u32) -> Option<Entity> {
    let mut query = world.query::<(Entity, &MaplibreMapIntegration)>();
    query
        .iter(world)
        .find(|(_, integration)| integration.id == integration_id)
        .map(|(entity, _)| entity)
}
