use utilities::glam_ext::bounding::{AxisAlignedBoundingBox2D, DAabb2};
use bevy::prelude::Reflect;
use glam::{DVec2, dvec2};
use std::f64::consts::PI;

pub const fn approx_lat_delta_from_len(len: f64) -> f64 {
    (len / 111320.0).to_radians()
}

fn approx_vertical_len(lat_min: f64, lat_max: f64) -> f64 {
    (lat_max - lat_min).to_degrees() * 111320.0
}

fn approx_lon_delta_from_len(lat: f64, len: f64) -> f64 {
    (len / (111320.0 * lat.cos())).to_radians()
}

fn approx_horizontal_len(lat: f64, lon_min: f64, lon_max: f64) -> f64 {
    (lon_max - lon_min).to_degrees() * 111320.0 * lat.cos()
}

pub fn approx_size_bound(gcs_area: DAabb2) -> DVec2 {
    dvec2(
        approx_horizontal_len(gcs_area.min().y, gcs_area.min().x, gcs_area.max().x).max(
            approx_horizontal_len(gcs_area.max().y, gcs_area.min().x, gcs_area.max().x),
        ),
        approx_vertical_len(gcs_area.min().y, gcs_area.max().y),
    )
}

pub trait Projection2D {
    fn gcs_bounds(&self) -> DAabb2;

    fn abs_bounds(&self) -> DAabb2 {
        DAabb2::new(
            self.gcs_to_abs(self.gcs_bounds().min()),
            self.gcs_to_abs(self.gcs_bounds().max()),
        )
    }

    fn gcs_to_abs(&self, gcs_pos: DVec2) -> DVec2;

    fn abs_to_gcs(&self, abs_pos: DVec2) -> DVec2;

    fn abs_to_rel(&self, abs_pos: DVec2) -> DVec2 {
        (abs_pos - self.abs_bounds().center()) / self.abs_bounds().size()
    }

    fn rel_to_abs(&self, rel_pos: DVec2) -> DVec2 {
        self.abs_bounds().center() + self.abs_bounds().size() * rel_pos
    }

    fn gcs_to_rel(&self, gcs_pos: DVec2) -> DVec2 {
        self.abs_to_rel(self.gcs_to_abs(gcs_pos))
    }

    fn rel_to_gcs(&self, rel_pos: DVec2) -> DVec2 {
        self.rel_to_abs(self.abs_to_gcs(rel_pos))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Reflect)]
pub struct BoundedMercatorProjection {
    pub lat_max: f64,
    pub lat_min: f64,
}

impl Projection2D for BoundedMercatorProjection {
    fn gcs_bounds(&self) -> DAabb2 {
        DAabb2::new(dvec2(-PI, self.lat_min), dvec2(PI, self.lat_max))
    }

    fn gcs_to_abs(&self, gcs_pos: DVec2) -> DVec2 {
        // debug_assert!(self.gcs_bounds().contains(gcs_pos));
        let gcs_pos = self.gcs_bounds().closest_point(gcs_pos);
        dvec2(gcs_pos.x, (PI / 4.0 + gcs_pos.y / 2.0).tan().ln())
    }

    fn abs_to_gcs(&self, abs_pos: DVec2) -> DVec2 {
        dvec2(abs_pos.x, abs_pos.y.sinh().atan())
    }
}
