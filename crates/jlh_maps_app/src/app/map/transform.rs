use crate::app::maplibre_gl_js::utils::mercator_coordinate::{LngLat, MercatorCoordinate};
use bevy::math::{DVec3, dvec3};

pub const MERCATOR_WORLD_SIZE: f64 = 100_000.0;

pub fn lng_lat_to_world(lng: f64, lat: f64, alt: f64) -> DVec3 {
    mercator_to_world(MercatorCoordinate::from_lng_lat(LngLat::new(lng, lat), alt))
}

fn mercator_to_world(coords: MercatorCoordinate) -> DVec3 {
    let MercatorCoordinate { x, y, z } = coords;
    dvec3(x, -y, z) * MERCATOR_WORLD_SIZE
}
