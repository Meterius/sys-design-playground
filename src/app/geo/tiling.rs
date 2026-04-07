use crate::app::geo::map::{
    Map, MapView, MapViewAbsLocalTransformChanged, MapViewWithMap, reposition_view,
};
use crate::app::settings::Settings;
use crate::app::utils::SoftExpect;
use crate::geo::coords::Projection2D;
use crate::geo::sub_division::{SubDivision2d, TileKey};
use crate::utils::glam_ext::bounding::{Aabb2, AxisAlignedBoundingBox2D, DAabb2};
use bevy::app::{App, Update};
use bevy::color::{Color, Luminance};
use bevy::prelude::{
    Added, ChildOf, Commands, Component, Entity, GlobalTransform, On, Query, Reflect, Res,
    Transform, Visibility, With, default,
};
use bevy::prelude::{Changed, IntoScheduleConfigs};
use bevy::prelude::{Plugin, ReflectComponent};
use bevy_inspector_egui::bevy_egui::{EguiContexts, EguiPrimaryContextPass};
use bevy_prototype_lyon::geometry::{ShapeBuilder, ShapeBuilderBase};
use bevy_prototype_lyon::shapes;
use bevy_vector_shapes::painter::ShapePainter;
use glam::{USizeVec2, Vec2, Vec3, vec2, vec3};
use std::collections::hash_map::Entry;
use std::collections::{HashMap, HashSet};
use crate::app::geo::tile_fetcher::{TileImageRequest, TileImageRequestWithMap, TileImageSprite};
use crate::geo::tiling::TileServerDataset;

pub struct MapViewTilingPlugin {}

impl Plugin for MapViewTilingPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                (
                    sync_tiles_for_view,
                    update_tiles.after(reposition_view),
                    setup_tiles,
                )
                    .chain(),
                draw_tiles_debug,
            ),
        );
        app.add_systems(EguiPrimaryContextPass, map_tiling_ui);
    }
}

#[derive(Component)]
pub struct MapViewTiling {
    target_tile_count: usize,
    tiles: HashMap<TileKey, Entity>,
}

impl MapViewTiling {
    pub fn new(target_tile_count: usize) -> Self {
        Self {
            target_tile_count,
            tiles: HashMap::new(),
        }
    }
}

fn sync_tiles_for_view(
    mut commands: Commands,
    tilings: Query<(&mut MapViewTiling, &MapViewTilingWithView)>,
    views: Query<(Entity, &MapView, &MapViewWithMap)>,
    maps: Query<&Map>,
) {
    for (mut tiling, &MapViewTilingWithView(view_id)) in tilings {
        if let Some((view_id, view, &MapViewWithMap(map_id))) =
            views.get(view_id).ok().soft_expect("")
            && let Some(map) = maps.get(map_id).ok().soft_expect("")
            && let Some(viewport_abs) = view.viewport_abs
        {
            let sub_div = SubDivision2d {
                area: map.projection.abs_bounds(),
            };

            let baseline_depth =
                sub_div.min_depth_for_tile_count(viewport_abs.size(), USizeVec2::new(1, 1));

            let target_depth = sub_div.min_depth_for_tile_count(
                viewport_abs.size(),
                USizeVec2::new(tiling.target_tile_count, tiling.target_tile_count),
            );

            let mut required_tile_keys = HashSet::new();

            for depth in baseline_depth..=target_depth {
                for tile in sub_div.tile_covering(viewport_abs, depth) {
                    required_tile_keys.insert(tile.key.clone());

                    if let Entry::Vacant(mut entry) = tiling.tiles.entry(tile.key.clone()) {
                        let tile_id = commands
                            .spawn(MapViewTile {
                                area_abs: tile.area,
                                key: tile.key.clone(),
                            })
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

fn map_tiling_ui(mut contexts: EguiContexts, settings: Res<Settings>) -> bevy::prelude::Result {
    if settings.debug_mode {
        egui::Window::new("Tiling").show(contexts.ctx_mut()?, |ui| {});
    }

    Ok(())
}

fn draw_tiles_debug(
    tiles: Query<(Entity, &MapViewTile, &ChildOf), Added<MapViewTile>>,
    views: Query<(&GlobalTransform, &MapView)>,
    mut painter: ShapePainter,
    settings: Res<Settings>,
) {
    if settings.debug_mode {
        for (tile_id, tile, &ChildOf(view_id)) in tiles {
            if let Some((view_transform, view)) = views.get(view_id).ok().soft_expect("") {}
        }
    }
}

fn update_tiles(
    mut tiles: Query<(Entity, &mut Transform, &MapViewTile, &ChildOf)>,
    views: Query<(&MapView)>,
) {
    for (tile_id, mut tile_transform, tile, &ChildOf(view_id)) in tiles {
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

fn setup_tiles(
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
                    area_local
                        .center()
                        .extend(1.0 - 0.5f32.powf(tile.key.len() as f32)),
                )
                .with_scale((Vec2::ONE * area_local.size().x).extend(1.0)),
                Visibility::default(),
            ));

            for (idx, dataset) in [
                TileServerDataset::GibsLayerModisTerraCorrectedReflectanceTrueColor,
                TileServerDataset::SenHubSentinel2L2a
            ].into_iter().enumerate() {
                let sprite_id = commands.spawn((
                    Transform::from_translation(vec3(0.0, 0.0, 0.1 + idx as f32)),
                    Visibility::default(),
                    TileImageRequest {
                        key: tile.key.clone(),
                        dataset,
                        priority: 0,
                    },
                    TileImageSprite {
                        size: Some(vec2(1.0, area_local.size().y / area_local.size().x))
                    },
                    TileImageRequestWithMap(map_id),
                )).id();

                commands.entity(tile_id).add_child(sprite_id);
            }

            let tile_hitbox_id = commands
                .spawn((
                    ShapeBuilder::with(&shapes::Rectangle {
                        extents: vec2(1.0, area_local.size().y / area_local.size().x),
                        ..default()
                    })
                    .fill(Color::WHITE.with_luminance(0.3))
                    .build(),
                    Transform::from_translation(Vec3::new(0.0, 0.0, 0.0)),
                ))
                .id();

            commands.entity(tile_id).add_child(tile_hitbox_id);

            commands.entity(tile_id).with_child((
                ShapeBuilder::with(&shapes::Rectangle {
                    extents: vec2(1.0, area_local.size().y / area_local.size().x),
                    ..default()
                })
                .stroke((
                    Color::WHITE.with_luminance(0.8),
                    0.1 / tile.key.len() as f32,
                ))
                .build(),
                Transform::from_translation(Vec3::new(0.0, 0.0, 500.0)),
            ));
        }
    }
}
