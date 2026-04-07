use crate::app::geo::map::{Map, MapView, MapViewWithMap, reposition_view};
use crate::app::geo::tile_fetcher::{
    TileImageRequest, TileImageRequestWithMap, TileImageSprite, handle_tile_image_sprite_loaded,
};
use crate::app::settings::Settings;
use crate::app::utils::SoftExpect;
use crate::geo::coords::Projection2D;
use crate::geo::sub_division::{SubDivision2d, TileKey};
use crate::geo::tiling::TileServerDataset;
use crate::utils::glam_ext::bounding::{Aabb2, AxisAlignedBoundingBox2D, DAabb2};
use bevy::app::{App, Update};
use bevy::color::Alpha;
use bevy::prelude::IntoScheduleConfigs;
use bevy::prelude::{
    Added, ChildOf, Commands, Component, Entity, Query, Reflect, Res, Sprite, Transform,
    Visibility, With,
};
use bevy::prelude::{Plugin, ReflectComponent};
use bevy_inspector_egui::bevy_egui::{EguiContexts, EguiPrimaryContextPass};
use bevy_pancam::PanCamSystems;
use glam::{USizeVec2, Vec2, dvec2, usizevec2, vec2, vec3};
use std::cmp::Ordering;
use std::collections::hash_map::Entry;
use std::collections::{HashMap, HashSet};

pub struct MapViewTilingPlugin {}

impl Plugin for MapViewTilingPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            ((
                sync_tiles_for_view.after(PanCamSystems),
                update_tiles.after(reposition_view),
                setup_tiles,
                sync_tile_fade.after(handle_tile_image_sprite_loaded),
            )
                .chain(),),
        );
        app.add_systems(EguiPrimaryContextPass, map_tiling_ui);
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
            && let Some(viewport_abs) = view.viewport_abs
        {
            let sub_div = SubDivision2d {
                area: map.projection.abs_bounds(),
            };

            let baseline_depth = sub_div
                .min_depth_for_tile_count(viewport_abs.size(), USizeVec2::new(1, 1))
                .saturating_sub(1);

            let target_depth = sub_div.min_depth_for_tile_count(
                viewport_abs.size(),
                USizeVec2::new(tiling.target_tile_count, tiling.target_tile_count),
            );

            let mut required_tile_keys = HashSet::new();

            let viewport_abs_expanded = viewport_abs.expand(dvec2(0.1, 0.1) * viewport_abs.size());
            for depth in baseline_depth..=(target_depth + 1) {
                for tile in sub_div.tile_covering(viewport_abs_expanded, depth) {
                    required_tile_keys.insert(tile.key.clone());

                    if let Entry::Vacant(entry) = tiling.tiles.entry(tile.key.clone()) {
                        let tile_id = commands
                            .spawn((
                                MapViewTile {
                                    area_abs: tile.area,
                                    key: tile.key.clone(),
                                },
                                MapViewTileWithTiling(tiling_id),
                            ))
                            .id();
                        commands.entity(view_id).add_child(tile_id);
                        entry.insert(tile_id);
                    }
                }
            }

            for (_, tile_id) in tiling
                .tiles
                .extract_if(|key, _| !required_tile_keys.contains(key))
            {
                commands.entity(tile_id).despawn();
            }

            let at_target_size = sub_div.area_size_for_min_depth_for_tile_count(
                target_depth,
                usizevec2(tiling.target_tile_count, tiling.target_tile_count),
            );
            let before_target_size = sub_div.area_size_for_min_depth_for_tile_count(
                target_depth.saturating_sub(1),
                usizevec2(tiling.target_tile_count, tiling.target_tile_count),
            );

            tiling.target_depth_fac = if target_depth != 0 {
                ((viewport_abs.size() - before_target_size) / (at_target_size - before_target_size))
                    .max_element() as f32
            } else {
                1.0
            };
            tiling.target_depth = target_depth;
        }
    }
}

#[derive(Component)]
#[relationship_target(relationship = MapViewTilingWithView)]
pub struct MapViewWithTilings(Vec<Entity>);

#[derive(Component)]
#[relationship(relationship_target = MapViewWithTilings)]
pub struct MapViewTilingWithView(pub Entity);

#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct MapViewTile {
    pub key: TileKey,
    pub area_abs: DAabb2,
}

#[derive(Component)]
#[relationship_target(relationship = MapViewTileWithTiling)]
pub struct MapViewTilingsWithTiles(Vec<Entity>);

#[derive(Component)]
#[relationship(relationship_target = MapViewTilingsWithTiles)]
pub struct MapViewTileWithTiling(pub Entity);

fn map_tiling_ui(mut contexts: EguiContexts, settings: Res<Settings>) -> bevy::prelude::Result {
    if settings.debug_mode {
        egui::Window::new("Tiling").show(contexts.ctx_mut()?, |_ui| {});
    }

    Ok(())
}

fn update_tiles(tiles: Query<(&mut Transform, &MapViewTile, &ChildOf)>, views: Query<&MapView>) {
    for (mut tile_transform, tile, &ChildOf(view_id)) in tiles {
        if let Some(view) = views.get(view_id).ok().soft_expect("") {
            let area_local = Aabb2::new(
                view.abs_to_local(tile.area_abs.min()),
                view.abs_to_local(tile.area_abs.max()),
            );

            tile_transform.translation = area_local.center().extend(tile_transform.translation.z);
            tile_transform.scale = (Vec2::ONE * area_local.size().x).extend(tile_transform.scale.z);
        }
    }
}

pub fn setup_tiles(
    mut commands: Commands,
    added_tiles: Query<(Entity, &MapViewTile, &ChildOf), Added<MapViewTile>>,
    views: Query<(&MapView, &MapViewWithMap)>,
) {
    for (tile_id, tile, &ChildOf(view_id)) in added_tiles {
        if let Some((view, &MapViewWithMap(map_id))) = views.get(view_id).ok().soft_expect("") {
            let area_local = Aabb2::new(
                view.abs_to_local(tile.area_abs.min()),
                view.abs_to_local(tile.area_abs.max()),
            );

            commands.entity(tile_id).insert((
                Transform::from_translation(
                    area_local.center().extend(10.0 * tile.key.len() as f32),
                )
                .with_scale((Vec2::ONE * area_local.size().x).extend(1.0)),
                Visibility::default(),
            ));

            for (idx, dataset) in [
                TileServerDataset::GibsLayerModisTerraCorrectedReflectanceTrueColor,
                TileServerDataset::SenHubSentinel2L2a,
            ]
            .into_iter()
            .enumerate()
            {
                let sprite_id = commands
                    .spawn((
                        Transform::from_translation(vec3(0.0, 0.0, 1.0 + idx as f32)),
                        Visibility::default(),
                        TileImageRequest {
                            key: tile.key.clone(),
                            dataset,
                            priority: -(tile.key.len() as isize),
                        },
                        TileImageSprite {
                            size: Some(vec2(1.0, area_local.size().y / area_local.size().x)),
                        },
                        MapViewTileFade {},
                        MapViewTileFadeWithTile(tile_id),
                        TileImageRequestWithMap(map_id),
                    ))
                    .id();

                commands.entity(tile_id).add_child(sprite_id);
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
