use crate::app::map::core::{MAP_VIEW_COLOR_RENDER_LAYER, MapViewSettings};
use crate::app::map::feature_plane_mesh::{
    FeatureMeshAltitudeConfig, FeatureTileBucket, FeatureTileBucketEdgeDistanceTexture,
    FeatureTileBucketPlaneMesh, handle_removed_features,
    setup_feature_tile_bucket_edge_distance_texture, setup_feature_tile_bucket_plane_mesh,
    tile_flat_center_world,
};
use crate::app::maplibre_gl_js::integration::MaplibreMapIntegration;
use crate::app::maplibre_gl_js::types::CanonicalTileId;
use crate::utils::debug::SoftExpect;
use bevy::asset::{Asset, Handle, load_internal_asset, uuid_handle};
use bevy::camera::visibility::RenderLayers;
use bevy::pbr::{ExtendedMaterial, MaterialExtension, MaterialPlugin, OpaqueRendererMethod};
use bevy::prelude::*;
use bevy::render::render_resource::AsBindGroup;
use bevy::shader::ShaderRef;
use big_space::grid::Grid;
use std::collections::HashMap;

pub struct WatersPlugin;

impl Plugin for WatersPlugin {
    fn build(&self, app: &mut App) {
        load_internal_asset!(
            app,
            WATER_GRADIENT_MATERIAL_SHADER_HANDLE,
            "../../../assets/shaders/water_gradient.fragment.wgsl",
            Shader::from_wgsl
        );
        app.add_plugins(MaterialPlugin::<WaterMaterial>::default())
            .add_systems(
                Update,
                (
                    sync_spawned_water_buckets,
                    setup_water_bucket_meshes,
                    update_water_material_time,
                )
                    .chain(),
            );
    }
}

const WATER_GRADIENT_MATERIAL_SHADER_HANDLE: Handle<Shader> =
    uuid_handle!("7b3c87e1-6c8c-4ae6-9c9f-9e3e271d8b90");
const WATER_SOURCE_LAYER: &str = "water";
const WATER_EDGE_DISTANCE_TEXTURE_RESOLUTION: UVec2 = UVec2::new(512, 512);

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
struct WaterTileBucket {
    material: Option<Handle<WaterMaterial>>,
}

type WaterMaterial = ExtendedMaterial<StandardMaterial, WaterMaterialExtension>;

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct WaterMaterialExtension {
    #[texture(100)]
    #[sampler(101)]
    pub edge_distance_texture: Handle<Image>,

    #[uniform(102)]
    pub params: Vec4,
}

impl MaterialExtension for WaterMaterialExtension {
    fn fragment_shader() -> ShaderRef {
        WATER_GRADIENT_MATERIAL_SHADER_HANDLE.into()
    }
}

fn sync_spawned_water_buckets(
    mut commands: Commands,
    map_view_settings: Res<MapViewSettings>,
    map_ints: Query<&MaplibreMapIntegration>,
    mut managers: Query<(Entity, &Grid, &mut WaterManager)>,
    mut buckets: Query<
        (
            &FeatureTileBucket,
            Option<&mut FeatureTileBucketPlaneMesh>,
            Option<&mut FeatureTileBucketEdgeDistanceTexture>,
        ),
        With<WaterTileBucket>,
    >,
    mut images: ResMut<Assets<Image>>,
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
            !map_view_settings.enable_waters,
        );

        if !map_view_settings.enable_waters {
            continue;
        }

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
                    &mut images,
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
    buckets: &mut Query<
        (
            &FeatureTileBucket,
            Option<&mut FeatureTileBucketPlaneMesh>,
            Option<&mut FeatureTileBucketEdgeDistanceTexture>,
        ),
        With<WaterTileBucket>,
    >,
    remove_all: bool,
) {
    spawned_waters.retain(|source_id, spawned_source| {
        let water_layer = (!remove_all).then_some(map_int).and_then(|map_int| {
            map_int
                .features
                .sources
                .get(source_id)
                .and_then(|source| source.source_layers.get(WATER_SOURCE_LAYER))
        });

        spawned_source.tiles.retain(|tile_id, bucket_entity| {
            if water_layer
                .and_then(|layer| layer.tiles.get(tile_id))
                .is_none()
            {
                commands.entity(*bucket_entity).despawn();
                return false;
            }

            let Ok((bucket, mut plane_mesh, mut edge_texture)) = buckets.get_mut(*bucket_entity)
            else {
                return true;
            };

            handle_removed_features(
                (!remove_all).then_some(map_int),
                bucket,
                plane_mesh.as_deref_mut(),
                edge_texture.as_deref_mut(),
            )
        });

        !spawned_source.tiles.is_empty()
    });
}

fn spawn_water_bucket(
    commands: &mut Commands,
    manager_id: Entity,
    maplibre_int_id: Entity,
    grid: &Grid,
    images: &mut Assets<Image>,
    source_id: &str,
    tile_id: CanonicalTileId,
) -> Entity {
    let center = tile_flat_center_world(tile_id);
    let (cell, translation) = grid.translation_to_grid(center.with_z(center.z));

    let bucket_id = commands
        .spawn((
            Name::new(format!("Water bucket {source_id}/{tile_id:?}")),
            cell,
            Transform::from_translation(translation),
            RenderLayers::layer(MAP_VIEW_COLOR_RENDER_LAYER),
            WaterTileBucket { material: None },
            FeatureTileBucket::new(
                maplibre_int_id,
                source_id,
                WATER_SOURCE_LAYER,
                tile_id,
                center,
            ),
            FeatureTileBucketPlaneMesh::default(),
            FeatureTileBucketEdgeDistanceTexture::new(
                WATER_EDGE_DISTANCE_TEXTURE_RESOLUTION,
                images,
            ),
        ))
        .id();

    commands.entity(manager_id).add_child(bucket_id);
    bucket_id
}

fn setup_water_bucket_meshes(
    mut commands: Commands,
    map_ints: Query<&MaplibreMapIntegration>,
    mut buckets: Query<
        (
            Entity,
            &mut WaterTileBucket,
            &FeatureTileBucket,
            &mut FeatureTileBucketPlaneMesh,
            &mut FeatureTileBucketEdgeDistanceTexture,
        ),
        With<WaterTileBucket>,
    >,
    mut meshes: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<WaterMaterial>>,
) {
    for (bucket_entity, mut water_bucket, bucket, mut plane_mesh, mut edge_texture) in
        buckets.iter_mut()
    {
        let Some(map_int) = map_ints.get(bucket.maplibre_int_id).ok() else {
            continue;
        };

        setup_feature_tile_bucket_edge_distance_texture(
            map_int,
            bucket,
            &mut edge_texture,
            &mut images,
        );
        let material = water_bucket
            .material
            .get_or_insert_with(|| {
                materials.add(ExtendedMaterial {
                    base: StandardMaterial {
                        base_color: Color::WHITE,
                        depth_bias: 40000.0,
                        ..default()
                    },
                    extension: WaterMaterialExtension {
                        edge_distance_texture: edge_texture.texture.clone(),
                        params: Vec4::ZERO,
                    },
                })
            })
            .clone();

        setup_feature_tile_bucket_plane_mesh(
            &mut commands,
            map_int,
            bucket_entity,
            bucket,
            &mut plane_mesh,
            &mut meshes,
            material,
            FeatureMeshAltitudeConfig {
                base_property_keys: None,
                top_property_keys: None,
            },
        );
    }
}

fn update_water_material_time(time: Res<Time>, mut materials: ResMut<Assets<WaterMaterial>>) {
    let elapsed = time.elapsed_secs();
    for (_, material) in materials.iter_mut() {
        material.extension.params.x = elapsed;
    }
}
