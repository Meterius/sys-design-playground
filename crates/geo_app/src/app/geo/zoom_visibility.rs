use crate::app::geo::map::MapViewContextRef;
use crate::app::geo::map::{MapView, MapViewContextQuery};
use crate::app::utils::debug::SoftExpect;
use crate::geo::coords::Projection2D;
use bevy::prelude::*;
use glam::DVec2;
use std::collections::HashMap;
use utilities::glam_ext::bounding::AxisAlignedBoundingBox2D;

pub struct MapZoomVisibilityPlugin {}

impl Plugin for MapZoomVisibilityPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PreUpdate, setup_map_view_zoom_info);
        app.add_systems(
            Update,
            (update_map_view_zoom_info, update_map_zoom_visibility).chain(),
        );
    }
}

#[derive(Component, Reflect)]
#[require(MapViewContextRef)]
struct MapViewZoomInfo {
    // percentage of viewport area relative to total absolute coordinates
    pub viewport_abs_perc: Option<DVec2>,
}

fn setup_map_view_zoom_info(mut commands: Commands, added_views: Query<Entity, Added<MapView>>) {
    for view_id in added_views {
        commands.entity(view_id).insert(MapViewZoomInfo {
            viewport_abs_perc: None,
        });
    }
}

fn update_map_view_zoom_info(
    views: Query<(Entity, &mut MapViewZoomInfo)>,
    view_ctx: MapViewContextQuery,
) {
    for (view_id, mut view_info) in views {
        if let Some(ctx) = view_ctx.get(view_id).soft_expect("") {
            view_info.viewport_abs_perc = ctx
                .view
                .viewport_abs
                .map(|viewport_abs| viewport_abs.size() / ctx.map.projection.abs_bounds().size());
        }
    }
}

#[derive(Component, Reflect)]
#[require(Visibility, MapViewContextRef)]
pub struct MapZoomVisibility {
    // Minimal and maximal percentage of view area relative to absolute coordinates at
    // which visibility is enabled
    pub visible_abs_view_perc: (f64, f64),
}

fn update_map_zoom_visibility(
    mut zooms: Query<(&mut Visibility, &MapZoomVisibility, &MapViewContextRef)>,
    zoom_infos: Query<(Entity, &MapViewZoomInfo)>,
) {
    let zoom_infos = zoom_infos.iter().collect::<HashMap<_, _>>();

    zooms.par_iter_mut().for_each(
        |(mut zoom_visibility, zoom, &MapViewContextRef { view_id })| {
            if let Some(view_id) = view_id.soft_expect("")
                && let Some(zoom_info) = zoom_infos.get(&view_id).soft_expect("")
            {
                let updated_visibility = if zoom_info.viewport_abs_perc.is_some_and(|perc| {
                    zoom.visible_abs_view_perc.0 <= perc.min_element()
                        && perc.max_element() <= zoom.visible_abs_view_perc.1
                }) {
                    Visibility::Inherited
                } else {
                    Visibility::Hidden
                };

                if updated_visibility != *zoom_visibility {
                    *zoom_visibility = updated_visibility;
                }
            }
        },
    );
}
