use crate::app::map::camera::MapViewCamera;
use crate::app::map::core::{MAP_VIEW_COLOR_RENDER_LAYER, MapViewSettings};
use crate::app::map::feature_plane_mesh::{
    FeatureMeshAltitudeConfig, FeaturePlaneMeshTileBucket, setup_feature_plane_mesh_tile_bucket,
    tile_flat_bounds_world,
};
use crate::app::maplibre_gl_js::integration::MaplibreMapIntegration;
use crate::app::maplibre_gl_js::types::CanonicalTileId;
use crate::utils::debug::SoftExpect;
use bevy::camera::visibility::RenderLayers;
use bevy::prelude::*;
use big_space::grid::Grid;
use std::collections::HashMap;

pub struct BuildingsPlugin;

impl Plugin for BuildingsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<BuildingMaterial>()
            .add_systems(
                Update,
                (
                    sync_spawned_building_buckets,
                    setup_visible_building_bucket_meshes,
                )
                    .chain(),
            )
            .add_systems(
                PostUpdate,
                sync_distance_visibility.after(TransformSystems::Propagate),
            );
    }
}

const DEFAULT_BUILDING_VISIBILITY_DISTANCE: f32 = 10.0;
const BUILDING_SOURCE_LAYER: &str = "building";
const BUILDING_BASE_ALTITUDE_PROPERTY_KEYS: &[&str] = &["render_min_height", "min_height"];
const BUILDING_TOP_ALTITUDE_PROPERTY_KEYS: &[&str] = &["render_height", "height"];

#[derive(Component)]
pub struct BuildingManager {
    pub maplibre_int_id: Entity,
    pub spawned_buildings: HashMap<String, SpawnedBuildingSource>,
}

#[derive(Default)]
pub struct SpawnedBuildingSource {
    tiles: HashMap<CanonicalTileId, Entity>,
}

#[derive(Component)]
struct BuildingTileBucket;

#[derive(Resource)]
struct BuildingMaterial(Handle<StandardMaterial>);

impl FromWorld for BuildingMaterial {
    fn from_world(world: &mut World) -> Self {
        let mut materials = world.resource_mut::<Assets<StandardMaterial>>();
        Self(materials.add(StandardMaterial {
            base_color: Color::srgb(0.8, 0.2, 0.2),
            ..default()
        }))
    }
}

fn sync_spawned_building_buckets(
    mut commands: Commands,
    map_view_settings: Res<MapViewSettings>,
    map_ints: Query<&MaplibreMapIntegration>,
    mut managers: Query<(Entity, &Grid, &mut BuildingManager)>,
    mut buckets: Query<&mut FeaturePlaneMeshTileBucket, With<BuildingTileBucket>>,
) {
    for (manager_id, grid, mut manager) in managers.iter_mut() {
        let maplibre_int_id = manager.maplibre_int_id;
        let Some(map_int) = map_ints.get(maplibre_int_id).ok().soft_expect("") else {
            continue;
        };

        remove_stale_building_buckets(
            &mut commands,
            map_int,
            &mut manager.spawned_buildings,
            &mut buckets,
            !map_view_settings.enable_buildings,
        );

        if !map_view_settings.enable_buildings {
            continue;
        }

        for (source_id, source) in &map_int.features.sources {
            let Some(building_layer) = source.source_layers.get(BUILDING_SOURCE_LAYER) else {
                continue;
            };

            for tile_id in building_layer.tiles.keys() {
                let spawned_source = manager
                    .spawned_buildings
                    .entry(source_id.clone())
                    .or_default();
                if spawned_source.tiles.contains_key(tile_id) {
                    continue;
                }

                let bucket_id = spawn_building_bucket(
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

fn remove_stale_building_buckets(
    commands: &mut Commands,
    map_int: &MaplibreMapIntegration,
    spawned_buildings: &mut HashMap<String, SpawnedBuildingSource>,
    buckets: &mut Query<&mut FeaturePlaneMeshTileBucket, With<BuildingTileBucket>>,
    remove_all: bool,
) {
    spawned_buildings.retain(|source_id, spawned_source| {
        let building_layer = (!remove_all).then_some(map_int).and_then(|map_int| {
            map_int
                .features
                .sources
                .get(source_id)
                .and_then(|source| source.source_layers.get(BUILDING_SOURCE_LAYER))
        });

        spawned_source.tiles.retain(|tile_id, bucket_entity| {
            let Some(current_features) = building_layer.and_then(|layer| layer.tiles.get(tile_id))
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

fn spawn_building_bucket(
    commands: &mut Commands,
    manager_id: Entity,
    maplibre_int_id: Entity,
    grid: &Grid,
    source_id: &str,
    tile_id: CanonicalTileId,
) -> Entity {
    let (center, flat_half_extents) = tile_flat_bounds_world(tile_id);
    let (cell, translation) = grid.translation_to_grid(center);

    let bucket_id = commands
        .spawn((
            Name::new(format!("Building bucket {source_id}/{tile_id:?}")),
            Visibility::Hidden,
            cell,
            Transform::from_translation(translation),
            RenderLayers::layer(MAP_VIEW_COLOR_RENDER_LAYER),
            DistanceVisibility {
                max_distance: DEFAULT_BUILDING_VISIBILITY_DISTANCE,
                flat_half_extents,
            },
            BuildingTileBucket,
            FeaturePlaneMeshTileBucket::new(
                maplibre_int_id,
                source_id,
                BUILDING_SOURCE_LAYER,
                tile_id,
                center,
            ),
        ))
        .id();

    commands.entity(manager_id).add_child(bucket_id);
    bucket_id
}

pub const DISABLE_DISTANCE_VISIBILITY: bool = false;

#[derive(Component)]
pub struct DistanceVisibility {
    pub max_distance: f32,
    pub flat_half_extents: Vec2,
}

fn sync_distance_visibility(
    cameras: Query<(&MapViewCamera, &GlobalTransform, &ChildOf)>,
    mut distance_visible_entities: Query<
        (
            &DistanceVisibility,
            &GlobalTransform,
            &ChildOf,
            &mut Visibility,
        ),
        Without<MapViewCamera>,
    >,
) {
    for (distance_visibility, entity_transform, ChildOf(entity_parent), mut visibility) in
        distance_visible_entities.iter_mut()
    {
        let Some((_, camera_transform, _)) = cameras
            .iter()
            .find(|(_, _, ChildOf(camera_parent))| camera_parent == entity_parent)
        else {
            continue;
        };

        let max_distance_squared =
            distance_visibility.max_distance * distance_visibility.max_distance;
        let distance_squared = distance_to_flat_bounds_squared(
            camera_transform.translation(),
            entity_transform.translation(),
            distance_visibility.flat_half_extents,
        );

        *visibility = if DISABLE_DISTANCE_VISIBILITY || distance_squared <= max_distance_squared {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
}

fn setup_visible_building_bucket_meshes(
    mut commands: Commands,
    map_ints: Query<&MaplibreMapIntegration>,
    mut buckets: Query<
        (Entity, &mut FeaturePlaneMeshTileBucket, &Visibility),
        With<BuildingTileBucket>,
    >,
    mut meshes: ResMut<Assets<Mesh>>,
    material: Res<BuildingMaterial>,
) {
    for (bucket_entity, mut bucket, visibility) in buckets.iter_mut() {
        if matches!(*visibility, Visibility::Hidden) {
            continue;
        }

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
                base_property_keys: Some(BUILDING_BASE_ALTITUDE_PROPERTY_KEYS),
                top_property_keys: Some(BUILDING_TOP_ALTITUDE_PROPERTY_KEYS),
            },
        );
    }
}

fn distance_to_flat_bounds_squared(point: Vec3, center: Vec3, half_extents: Vec2) -> f32 {
    let local = point - center;
    let outside_xy = (local.xy().abs() - half_extents).max(Vec2::ZERO);

    outside_xy.length_squared() + local.z * local.z
}
