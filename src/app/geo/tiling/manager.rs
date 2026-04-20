use crate::app::geo::map::MapViewContextRef;
use crate::app::geo::map::{Map, MapView, MapViewContextQuery, MapViewWithMap};
use crate::app::geo::tiling::requests::{TileImageRequest, TileRequestManagersByDataset};
use crate::app::geo::tiling::sprite::{TileImageSprite, handle_tile_image_sprite_loaded};
use crate::app::utils::async_requests::RequestWithManager;
use crate::app::utils::big_space_ext::CommandsWithSpatial;
use crate::app::utils::debug::SoftExpect;
use crate::geo::coords::Projection2D;
use backend_model::earth_tiling_service_model::{Layer, LocalLayer};
use bevy::app::{App, Update};
use bevy::camera::visibility::RenderLayers;
use bevy::color::Alpha;
use bevy::picking::Pickable;
use bevy::prelude::{
    Added, Commands, Component, Entity, IntoScheduleConfigs, Name, Query, Reflect, Res, Sprite,
    Transform, Visibility, With,
};
use bevy::prelude::{Plugin, ReflectComponent};
use bevy_pancam::PanCamSystems;
use glam::{USizeVec2, dvec2, usizevec2, vec3};
use std::cmp::Ordering;
use std::collections::hash_map::Entry;
use std::collections::{HashMap, HashSet};
use utilities::glam_ext::bounding::{Aabb2, AxisAlignedBoundingBox2D, DAabb2};
use utilities::glam_ext::sub_division::{SubDivision2d, TileKey, tile_key_str};

pub struct TilingMangerPlugin {}

impl Plugin for TilingMangerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            ((
                sync_tiles_for_view.after(PanCamSystems),
                setup_tiles,
                sync_tile_fade.after(handle_tile_image_sprite_loaded),
            )
                .chain(),),
        );
    }
}

#[derive(Component, Reflect)]
pub struct MapViewTiling {
    pub target_tile_count: usize,
    pub target_depth_fac: f32,
    pub target_depth: usize,
    tiles: HashMap<TileKey, Entity>,
}

impl MapViewTiling {
    pub fn new(target_tile_count: usize) -> Self {
        Self {
            target_tile_count,
            target_depth_fac: 0.0,
            target_depth: 0,
            tiles: HashMap::new(),
        }
    }
}

fn sync_tiles_for_view(
    mut commands: Commands,
    tilings: Query<(Entity, &mut MapViewTiling, &MapViewTilingWithView)>,
    views: Query<(Entity, &MapView, &MapViewWithMap)>,
    maps: Query<&Map>,
) {
    for (tiling_id, mut tiling, &MapViewTilingWithView(view_id)) in tilings {
        if let Some((view_id, view, &MapViewWithMap(map_id))) =
            views.get(view_id).ok().soft_expect("")
            && let Some(map) = maps.get(map_id).ok().soft_expect("")
        {
            let sub_div = SubDivision2d {
                area: map.projection.abs_bounds(),
            };

            let mut required_tile_keys = HashSet::new();

            if let Some(viewport_abs) = view.viewport_abs {
                let baseline_depth = sub_div
                    .min_depth_for_tile_count(viewport_abs.size(), USizeVec2::new(1, 1))
                    .saturating_sub(2);

                let target_depth = sub_div.min_depth_for_tile_count(
                    viewport_abs.size(),
                    USizeVec2::new(tiling.target_tile_count, tiling.target_tile_count),
                );

                let viewport_abs_expanded =
                    viewport_abs.expand(dvec2(0.1, 0.1) * viewport_abs.size());
                for depth in baseline_depth..=(target_depth) {
                    for tile in sub_div.tile_covering(viewport_abs_expanded, depth) {
                        required_tile_keys.insert(tile.key.clone());

                        if let Entry::Vacant(entry) = tiling.tiles.entry(tile.key.clone()) {
                            let tile_id = commands
                                .spawn_spatial((
                                    Name::new(format!("Tile {}", tile_key_str(&tile.key))),
                                    MapViewTile {
                                        area_abs: tile.area,
                                        key: tile.key.clone(),
                                    },
                                    MapViewTileWithTiling(tiling_id),
                                ))
                                .id();
                            commands.entity(tiling_id).add_child(tile_id);
                            entry.insert(tile_id);
                        }
                    }
                }

                tiling.target_depth_fac = if target_depth != 0 {
                    let at_target_size = sub_div.area_size_for_min_depth_for_tile_count(
                        target_depth,
                        usizevec2(tiling.target_tile_count, tiling.target_tile_count),
                    );

                    let before_target_size = sub_div.area_size_for_min_depth_for_tile_count(
                        target_depth.saturating_sub(1),
                        usizevec2(tiling.target_tile_count, tiling.target_tile_count),
                    );

                    ((viewport_abs.size() - before_target_size)
                        / (at_target_size - before_target_size))
                        .max_element() as f32
                } else {
                    1.0
                };
                tiling.target_depth = target_depth;
            } else {
                tiling.target_depth_fac = 1.0;
                tiling.target_depth = 0;
            }

            for (_, tile_id) in tiling
                .tiles
                .extract_if(|key, _| !required_tile_keys.contains(key))
            {
                commands.entity(tile_id).despawn();
            }
        }
    }
}

#[derive(Component, Reflect)]
#[relationship_target(relationship = MapViewTilingWithView)]
pub struct MapViewWithTilings(Vec<Entity>);

#[derive(Component, Reflect)]
#[relationship(relationship_target = MapViewWithTilings)]
pub struct MapViewTilingWithView(pub Entity);

#[derive(Component, Debug, Reflect)]
#[reflect(Component)]
#[require(MapViewContextRef)]
pub struct MapViewTile {
    pub key: TileKey,
    pub area_abs: DAabb2,
}

#[derive(Component, Reflect)]
#[relationship_target(relationship = MapViewTileWithTiling)]
pub struct MapViewTilingsWithTiles(Vec<Entity>);

#[derive(Component, Reflect)]
#[relationship(relationship_target = MapViewTilingsWithTiles)]
pub struct MapViewTileWithTiling(pub Entity);

pub fn setup_tiles(
    req_managers_by_layer: Option<Res<TileRequestManagersByDataset>>,
    mut commands: Commands,
    added_tiles: Query<(Entity, &MapViewTile), Added<MapViewTile>>,
    map_view_context: MapViewContextQuery,
) {
    for (tile_id, tile) in added_tiles {
        if let Some(ctx) = map_view_context.get(tile_id) {
            let area_local = Aabb2::new(
                ctx.view.abs_to_local(tile.area_abs.min()).as_vec2(),
                ctx.view.abs_to_local(tile.area_abs.max()).as_vec2(),
            );

            commands.entity(tile_id).insert((
                Transform::from_translation(
                    area_local
                        .center()
                        .extend(0.1 + 0.1 * tile.key.len() as f32),
                )
                .with_scale(vec3(1.0, 1.0, 0.01)),
                Visibility::default(),
            ));

            for (idx, layer) in [Layer::Local(LocalLayer::GlobalMosaicSen2)]
                .into_iter()
                .enumerate()
            {
                if let Some(&req_manager_id) = req_managers_by_layer
                    .as_ref()
                    .and_then(|m| m.managers.get(&layer))
                    .soft_expect("")
                {
                    let sprite_id = commands
                        .spawn((
                            Transform::from_translation(vec3(0.0, 0.0, 1.0 + idx as f32)),
                            Visibility::default(),
                            TileImageRequest::new(
                                (ctx.map.projection, tile.key.clone()),
                                -(tile.key.len() as isize),
                            ),
                            RequestWithManager(req_manager_id),
                            TileImageSprite {
                                size: Some(area_local.size()),
                            },
                            MapViewTileFade {},
                            MapViewTileFadeWithTile(tile_id),
                            Pickable::default(),
                            RenderLayers::layer(1),
                        ))
                        .id();

                    commands.entity(tile_id).add_child(sprite_id);
                }
            }
        }
    }
}

#[derive(Component, Reflect)]
pub struct MapViewTileFade {}

#[derive(Component, Reflect)]
#[relationship(relationship_target = MapViewTileWithFades)]
pub struct MapViewTileFadeWithTile(pub Entity);

#[derive(Component, Reflect)]
#[relationship_target(relationship = MapViewTileFadeWithTile)]
pub struct MapViewTileWithFades(Vec<Entity>);

fn sync_tile_fade(
    fades: Query<(&mut Sprite, &MapViewTileFadeWithTile), With<MapViewTileFade>>,
    tiles: Query<(&MapViewTile, &MapViewTileWithTiling)>,
    tilings: Query<&MapViewTiling>,
) {
    for (mut sprite, &MapViewTileFadeWithTile(tile_id)) in fades {
        if let Some((tile, &MapViewTileWithTiling(tiling_id))) =
            tiles.get(tile_id).ok().soft_expect("")
            && let Some(tiling) = tilings.get(tiling_id).ok().soft_expect("")
        {
            sprite
                .color
                .set_alpha(match tile.key.len().cmp(&tiling.target_depth) {
                    Ordering::Less => 1.0,
                    Ordering::Equal => tiling.target_depth_fac,
                    Ordering::Greater => 0.0,
                });
        }
    }
}
