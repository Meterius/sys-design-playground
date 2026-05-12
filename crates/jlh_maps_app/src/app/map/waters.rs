use crate::app::map::core::MAP_VIEW_COLOR_RENDER_LAYER;
use crate::app::map::feature_plane_mesh::{
    FeatureMeshAltitudeConfig, FeaturePlaneMeshTileBucket, setup_feature_plane_mesh_tile_bucket,
    tile_flat_center_world,
};
use crate::app::maplibre_gl_js::integration::MaplibreMapIntegration;
use crate::app::maplibre_gl_js::types::CanonicalTileId;
use crate::utils::debug::SoftExpect;
use bevy::camera::visibility::RenderLayers;
use bevy::prelude::*;
use big_space::grid::Grid;
use std::collections::HashMap;

pub struct WatersPlugin;

impl Plugin for WatersPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<WaterMaterial>().add_systems(
            Update,
            (sync_spawned_water_buckets, setup_water_bucket_meshes).chain(),
        );
    }
}

const WATER_SOURCE_LAYER: &str = "water";

#[derive(Component)]
pub struct WaterManager {
    pub maplibre_int_id: Entity,
    pub spawned_waters: HashMap<String, SpawnedWaterSource>,
}

#[derive(Default)]
pub struct SpawnedWaterSource {
    tiles: HashMap<CanonicalTileId, Entity>,
}

#[derive(Component)]
struct WaterTileBucket;

#[derive(Resource)]
struct WaterMaterial(Handle<StandardMaterial>);

impl FromWorld for WaterMaterial {
    fn from_world(world: &mut World) -> Self {
        let mut materials = world.resource_mut::<Assets<StandardMaterial>>();
        Self(materials.add(StandardMaterial {
            base_color: Color::srgb(0.2, 0.2, 0.8),
            ..default()
        }))
    }
}

fn sync_spawned_water_buckets(
    mut commands: Commands,
    map_ints: Query<&MaplibreMapIntegration>,
    mut managers: Query<(Entity, &Grid, &mut WaterManager)>,
    mut buckets: Query<&mut FeaturePlaneMeshTileBucket, With<WaterTileBucket>>,
) {
    for (manager_id, grid, mut manager) in managers.iter_mut() {
        let maplibre_int_id = manager.maplibre_int_id;
        let Some(map_int) = map_ints.get(maplibre_int_id).ok().soft_expect("") else {
            continue;
        };

        remove_stale_water_buckets(
            &mut commands,
            map_int,
            &mut manager.spawned_waters,
            &mut buckets,
        );

        for (source_id, source) in &map_int.features.sources {
            let Some(water_layer) = source.source_layers.get(WATER_SOURCE_LAYER) else {
                continue;
            };

            for tile_id in water_layer.tiles.keys() {
                let spawned_source = manager.spawned_waters.entry(source_id.clone()).or_default();
                if spawned_source.tiles.contains_key(tile_id) {
                    continue;
                }

                let bucket_id = spawn_water_bucket(
                    &mut commands,
                    manager_id,
                    maplibre_int_id,
                    grid,
                    source_id,
                    *tile_id,
                );
                spawned_source.tiles.insert(*tile_id, bucket_id);
            }
        }
    }
}

fn remove_stale_water_buckets(
    commands: &mut Commands,
    map_int: &MaplibreMapIntegration,
    spawned_waters: &mut HashMap<String, SpawnedWaterSource>,
    buckets: &mut Query<&mut FeaturePlaneMeshTileBucket, With<WaterTileBucket>>,
) {
    spawned_waters.retain(|source_id, spawned_source| {
        let water_layer = map_int
            .features
            .sources
            .get(source_id)
            .and_then(|source| source.source_layers.get(WATER_SOURCE_LAYER));

        spawned_source.tiles.retain(|tile_id, bucket_entity| {
            let Some(current_features) = water_layer.and_then(|layer| layer.tiles.get(tile_id))
            else {
                commands.entity(*bucket_entity).despawn();
                return false;
            };

            let Ok(mut bucket) = buckets.get_mut(*bucket_entity) else {
                return true;
            };

            bucket.handle_removed_features(current_features);

            true
        });

        !spawned_source.tiles.is_empty()
    });
}

fn spawn_water_bucket(
    commands: &mut Commands,
    manager_id: Entity,
    maplibre_int_id: Entity,
    grid: &Grid,
    source_id: &str,
    tile_id: CanonicalTileId,
) -> Entity {
    let center = tile_flat_center_world(tile_id);
    let (cell, translation) = grid.translation_to_grid(center.with_z(center.z + 0.0001));

    let bucket_id = commands
        .spawn((
            Name::new(format!("Water bucket {source_id}/{tile_id:?}")),
            cell,
            Transform::from_translation(translation),
            RenderLayers::layer(MAP_VIEW_COLOR_RENDER_LAYER),
            WaterTileBucket,
            FeaturePlaneMeshTileBucket::new(
                maplibre_int_id,
                source_id,
                WATER_SOURCE_LAYER,
                tile_id,
                center,
            ),
        ))
        .id();

    commands.entity(manager_id).add_child(bucket_id);
    bucket_id
}

fn setup_water_bucket_meshes(
    mut commands: Commands,
    map_ints: Query<&MaplibreMapIntegration>,
    mut buckets: Query<(Entity, &mut FeaturePlaneMeshTileBucket), With<WaterTileBucket>>,
    mut meshes: ResMut<Assets<Mesh>>,
    material: Res<WaterMaterial>,
) {
    for (bucket_entity, mut bucket) in buckets.iter_mut() {
        let Some(map_int) = map_ints.get(bucket.maplibre_int_id).ok() else {
            continue;
        };

        setup_feature_plane_mesh_tile_bucket(
            &mut commands,
            map_int,
            bucket_entity,
            &mut bucket,
            &mut meshes,
            material.0.clone(),
            FeatureMeshAltitudeConfig {
                base_property_keys: None,
                top_property_keys: None,
            },
        );
    }
}
