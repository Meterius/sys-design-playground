// Rust port of https://github.com/maplibre/maplibre-gl-js/blob/4412ee1bd912f669534fc801aaae87aaf8066274/src/geo/mercator_coordinate.ts

/// MapLibre's WGS-84 mean earth radius in meters.
pub const EARTH_RADIUS: f64 = 6_371_008.8;

/// The average circumference of the world in meters.
pub const EARTH_CIRCUMFERENCE: f64 = 2.0 * std::f64::consts::PI * EARTH_RADIUS;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LngLat {
    pub lng: f64,
    pub lat: f64,
}

impl LngLat {
    pub const fn new(lng: f64, lat: f64) -> Self {
        Self { lng, lat }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MercatorCoordinate {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl MercatorCoordinate {
    pub const fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }

    pub fn from_lng_lat(lng_lat: LngLat, altitude: f64) -> Self {
        Self {
            x: mercator_x_from_lng(lng_lat.lng),
            y: mercator_y_from_lat(lng_lat.lat),
            z: mercator_z_from_altitude(altitude, lng_lat.lat),
        }
    }

    pub fn to_lng_lat(self) -> LngLat {
        LngLat {
            lng: lng_from_mercator_x(self.x),
            lat: lat_from_mercator_y(self.y),
        }
    }

    pub fn to_altitude(self) -> f64 {
        altitude_from_mercator_z(self.z, self.y)
    }

    pub fn meter_in_mercator_coordinate_units(self) -> f64 {
        (1.0 / EARTH_CIRCUMFERENCE) * mercator_scale(lat_from_mercator_y(self.y))
    }
}

pub fn circumference_at_latitude(latitude: f64) -> f64 {
    EARTH_CIRCUMFERENCE * latitude.to_radians().cos()
}

pub fn mercator_x_from_lng(lng: f64) -> f64 {
    (180.0 + lng) / 360.0
}

pub fn mercator_y_from_lat(lat: f64) -> f64 {
    let mercator_y = 180.0
        - (180.0 / std::f64::consts::PI)
            * ((std::f64::consts::PI / 4.0) + (lat * std::f64::consts::PI / 360.0))
                .tan()
                .ln();

    mercator_y / 360.0
}

pub fn mercator_z_from_altitude(altitude: f64, lat: f64) -> f64 {
    altitude / circumference_at_latitude(lat)
}

pub fn lng_from_mercator_x(x: f64) -> f64 {
    x * 360.0 - 180.0
}

pub fn lat_from_mercator_y(y: f64) -> f64 {
    let y2 = 180.0 - y * 360.0;
    (360.0 / std::f64::consts::PI) * (y2 * std::f64::consts::PI / 180.0).exp().atan() - 90.0
}

pub fn altitude_from_mercator_z(z: f64, y: f64) -> f64 {
    z * circumference_at_latitude(lat_from_mercator_y(y))
}

pub fn mercator_scale(lat: f64) -> f64 {
    1.0 / lat.to_radians().cos()
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPSILON: f64 = 1e-12;

    fn assert_approx_eq(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() <= EPSILON,
            "expected {actual} to approximately equal {expected}"
        );
    }

    #[test]
    fn projects_null_island() {
        let coordinate = MercatorCoordinate::from_lng_lat(LngLat::new(0.0, 0.0), 0.0);

        assert_approx_eq(coordinate.x, 0.5);
        assert_approx_eq(coordinate.y, 0.5);
        assert_approx_eq(coordinate.z, 0.0);
    }

    #[test]
    fn converts_altitude_using_latitude_scale() {
        let coordinate = MercatorCoordinate::new(0.0, 0.0, 0.02);

        assert_approx_eq(coordinate.to_altitude(), 69_065.545_611_890_78);
    }

    #[test]
    fn round_trips_lng_lat() {
        let lng_lat = LngLat::new(12.5, 48.25);
        let coordinate = MercatorCoordinate::from_lng_lat(lng_lat, 123.0);
        let round_trip = coordinate.to_lng_lat();

        assert_approx_eq(round_trip.lng, lng_lat.lng);
        assert_approx_eq(round_trip.lat, lng_lat.lat);
        assert_approx_eq(coordinate.to_altitude(), 123.0);
    }
}
