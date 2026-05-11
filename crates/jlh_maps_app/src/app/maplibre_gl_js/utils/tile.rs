use crate::app::map::transform::lng_lat_to_world;
use crate::app::maplibre_gl_js::types::CanonicalTileId;
use crate::app::maplibre_gl_js::utils::mercator_coordinate::{
    lat_from_mercator_y, lng_from_mercator_x,
};
use bevy::math::{DVec2, DVec3, Vec3Swizzles, dvec2};

pub fn get_tile_lnglat_bounds(id: CanonicalTileId) -> (DVec2, DVec2) {
    let ll_min = dvec2(
        lng_from_mercator_x(id.x as f64 / 2f64.powf(id.z as f64)),
        lat_from_mercator_y((id.y as f64 + 1.0) / 2f64.powf(id.z as f64)),
    );

    let ll_max = dvec2(
        lng_from_mercator_x((id.x + 1) as f64 / 2f64.powf(id.z as f64)),
        lat_from_mercator_y(id.y as f64 / 2f64.powf(id.z as f64)),
    );

    (ll_min.min(ll_max), ll_min.max(ll_max))
}

pub fn tile_transform_d(tile_id: CanonicalTileId, alt: f64) -> (DVec3, DVec2) {
    let bounds = get_tile_lnglat_bounds(tile_id);

    let south_west = lng_lat_to_world(bounds.0.x, bounds.0.y, alt);
    let north_east = lng_lat_to_world(bounds.1.x, bounds.1.y, alt);

    let min = south_west.min(north_east);
    let max = south_west.max(north_east);
    let size = max - min;

    ((min + max) * 0.5, size.xy())
}
