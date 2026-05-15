use crate::app::map::camera::MapViewCamera;
use crate::app::map::core::{MAP_VIEW_COLOR_RENDER_LAYER, MapViewSettings};
use crate::app::map::feature_plane_mesh::{
    FeatureTileBucket, FeatureTileBucketPlaneMesh, FeatureTileBucketPlaneMeshConfig,
    tile_flat_bounds_world,
};
use crate::app::maplibre_gl_js::integration::MaplibreMapIntegration;
use crate::app::maplibre_gl_js::types::CanonicalTileId;
use crate::utils::debug::SoftExpect;
use bevy::asset::{Asset, AssetApp, Handle, load_internal_asset, uuid_handle};
use bevy::camera::visibility::RenderLayers;
use bevy::pbr::{
    DefaultOpaqueRendererMethod, ExtendedMaterial, MaterialExtension, MaterialPlugin,
    OpaqueRendererMethod,
};
use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, ShaderType};
use bevy::shader::ShaderRef;
use big_space::grid::Grid;
use std::collections::HashMap;

pub struct BuildingsPlugin;

impl Plugin for BuildingsPlugin {
    fn build(&self, app: &mut App) {
        load_internal_asset!(
            app,
            BUILDING_MATERIAL_SHADER_HANDLE,
            "../../../assets/shaders/building_pbr.fragment.wgsl",
            Shader::from_wgsl
        );
        app.register_type::<BuildingMaterialUniform>()
            .register_type::<BuildingMaterialExtension>()
            .register_asset_reflect::<BuildingMaterial>()
            .add_plugins(MaterialPlugin::<BuildingMaterial>::default())
            .init_resource::<GlobalBuildingMaterial>()
            .add_systems(PreUpdate, sync_building_material_opaque_render_method)
            .add_systems(Update, (sync_spawned_building_buckets,).chain())
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
const BUILDING_MATERIAL_SHADER_HANDLE: Handle<Shader> =
    uuid_handle!("6821f839-72cf-4b53-a709-d0260d921b72");

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

#[derive(Resource, Reflect)]
struct GlobalBuildingMaterial(Handle<BuildingMaterial>);

type BuildingMaterial = ExtendedMaterial<StandardMaterial, BuildingMaterialExtension>;

#[derive(ShaderType, Reflect, Debug, Clone, Copy)]
struct BuildingMaterialUniform {
    height_gradient_strength: f32,
    height_gradient_upper_altitude: f32,

    base_shadow_strength: f32,
    base_shadow_upper_altitude: f32,

    lambert_tint_strength: f32,
    lambert_shade_strength: f32,
    _webgl2_padding_24b: u32,
    _webgl2_padding_28b: u32,
}

#[derive(Asset, AsBindGroup, Reflect, Debug, Clone)]
struct BuildingMaterialExtension {
    #[uniform(100)]
    uniform: BuildingMaterialUniform,
}

impl MaterialExtension for BuildingMaterialExtension {
    fn fragment_shader() -> ShaderRef {
        BUILDING_MATERIAL_SHADER_HANDLE.into()
    }

    fn deferred_fragment_shader() -> ShaderRef {
        BUILDING_MATERIAL_SHADER_HANDLE.into()
    }
}

impl GlobalBuildingMaterial {
    fn material() -> BuildingMaterial {
        ExtendedMaterial {
            base: StandardMaterial {
                base_color: Color::hsv(20., 0.08, 0.76),
                perceptual_roughness: 0.8,
                reflectance: 0.05,
                opaque_render_method: OpaqueRendererMethod::Auto,
                ..default()
            },
            extension: BuildingMaterialExtension {
                uniform: BuildingMaterialUniform {
                    height_gradient_strength: 0.25,
                    height_gradient_upper_altitude: 40.0,
                    base_shadow_strength: 0.1,
                    base_shadow_upper_altitude: 3.0,
                    lambert_tint_strength: 0.2,
                    lambert_shade_strength: 0.1,
                    _webgl2_padding_24b: 0,
                    _webgl2_padding_28b: 0,
                },
            },
        }
    }
}

impl FromWorld for GlobalBuildingMaterial {
    fn from_world(world: &mut World) -> Self {
        let mut materials = world.resource_mut::<Assets<BuildingMaterial>>();
        Self(materials.add(GlobalBuildingMaterial::material()))
    }
}

fn sync_building_material_opaque_render_method(
    default_opaque_renderer_method: Res<DefaultOpaqueRendererMethod>,
    handle: Res<GlobalBuildingMaterial>,
    mut materials: ResMut<Assets<BuildingMaterial>>,
) {
    if default_opaque_renderer_method.is_changed()
        && let Some(material) = materials.get_mut(&handle.0) {
            *material = GlobalBuildingMaterial::material();
        }
}

fn sync_spawned_building_buckets(
    mut commands: Commands,
    map_view_settings: Res<MapViewSettings>,
    map_ints: Query<&MaplibreMapIntegration>,
    mut managers: Query<(Entity, &Grid, &mut BuildingManager)>,
    buckets: Query<(), With<BuildingTileBucket>>,
    material: Res<GlobalBuildingMaterial>,
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
            &buckets,
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
                    material.0.clone(),
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
    buckets: &Query<(), With<BuildingTileBucket>>,
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
            if building_layer
                .and_then(|layer| layer.tiles.get(tile_id))
                .is_none()
            {
                commands.entity(*bucket_entity).despawn();
                return false;
            }

            buckets.get(*bucket_entity).is_ok()
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
    material: Handle<BuildingMaterial>,
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
            FeatureTileBucket::new(
                maplibre_int_id,
                source_id,
                BUILDING_SOURCE_LAYER,
                tile_id,
                center,
            ),
            FeatureTileBucketPlaneMesh::default(),
            FeatureTileBucketPlaneMeshConfig {
                base_property_keys: Some(BUILDING_BASE_ALTITUDE_PROPERTY_KEYS),
                top_property_keys: Some(BUILDING_TOP_ALTITUDE_PROPERTY_KEYS),
            },
            MeshMaterial3d(material),
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

fn distance_to_flat_bounds_squared(point: Vec3, center: Vec3, half_extents: Vec2) -> f32 {
    let local = point - center;
    let outside_xy = (local.xy().abs() - half_extents).max(Vec2::ZERO);

    outside_xy.length_squared() + local.z * local.z
}
