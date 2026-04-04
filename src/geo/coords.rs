use bevy::prelude::Vec2;
use std::f32::consts::PI;
use std::ops::Rem;

#[derive(Debug, Clone)]
pub struct LonLatVec2 {
    pub x: f32,
    pub y: f32,
}

impl From<Vec2> for LonLatVec2 {
    fn from(value: Vec2) -> Self {
        Self {
            x: value.x,
            y: value.y,
        }
    }
}

impl From<LonLatVec2> for Vec2 {
    fn from(val: LonLatVec2) -> Self {
        Vec2::new(val.x, val.y)
    }
}

#[derive(Debug, Clone)]
pub struct RadLonLatVec2 {
    pub x: f32,
    pub y: f32,
}

impl RadLonLatVec2 {
    pub fn rem_euclid(&self) -> RadLonLatVec2 {
        RadLonLatVec2 {
            x: self.x.rem_euclid(2.0 * PI),
            y: self.y.rem_euclid(PI),
        }
    }

    pub fn rem(&self) -> RadLonLatVec2 {
        RadLonLatVec2 {
            x: self.x.rem(2.0 * PI),
            y: self.y.rem(PI),
        }
    }
}

fn approx_vertical_len(lat_min: f32, lat_max: f32) -> f32 {
    (lat_max - lat_min) * (360.0 / 2.0 * PI) * 111320.0
}

fn approx_horizontal_len(lat: f32, lon_min: f32, lon_max: f32) -> f32 {
    (lon_max - lon_min) * 360.0 / (2.0 * PI) * 111320.0 * lat.cos()
}

pub fn approx_size_bound((gcs_min, gcs_max): &(RadLonLatVec2, RadLonLatVec2)) -> Vec2 {
    Vec2::new(
        approx_horizontal_len(gcs_max.y, gcs_min.x, gcs_max.x).max(
        approx_horizontal_len(gcs_min.y, gcs_min.x, gcs_max.x)),
        approx_vertical_len(gcs_min.y, gcs_max.y),
    )
}

impl From<Vec2> for RadLonLatVec2 {
    fn from(value: Vec2) -> Self {
        Self {
            x: value.x,
            y: value.y,
        }
    }
}

impl From<RadLonLatVec2> for Vec2 {
    fn from(val: RadLonLatVec2) -> Self {
        Vec2::new(val.x, val.y)
    }
}

impl From<LonLatVec2> for RadLonLatVec2 {
    fn from(value: LonLatVec2) -> Self {
        Self {
            x: value.x.to_radians(),
            y: value.y.to_radians(),
        }
    }
}

impl From<RadLonLatVec2> for LonLatVec2 {
    fn from(val: RadLonLatVec2) -> Self {
        LonLatVec2 {
            x: val.x.to_degrees(),
            y: val.y.to_degrees(),
        }
    }
}

pub trait Projection2D {
    fn abs_pos(&self) -> Vec2;

    fn abs_size(&self) -> Vec2;

    fn gcs_to_abs(&self, pos: &RadLonLatVec2) -> Vec2;

    fn gcs_to_rel(&self, pos: &RadLonLatVec2) -> Vec2 {
        (self.gcs_to_abs(pos) - self.abs_pos()) / self.abs_size()
    }

    fn abs_to_gcs(&self, pos: &Vec2) -> RadLonLatVec2;

    fn rel_to_gcs(&self, pos: &Vec2) -> RadLonLatVec2 {
        self.abs_to_gcs(&(self.abs_pos() + pos * self.abs_size()))
    }
}

#[derive(Debug, Clone)]
pub struct BoundedMercatorProjection {
    pub lat_max: f32,
    pub lat_min: f32,
}

impl Projection2D for BoundedMercatorProjection {
    fn abs_pos(&self) -> Vec2 {
        self.gcs_to_abs(&RadLonLatVec2 { x: 0.0, y: 0.0 })
    }
    fn abs_size(&self) -> Vec2 {
        self.gcs_to_abs(&RadLonLatVec2 {
            x: PI,
            y: self.lat_max,
        }) - self.gcs_to_abs(&RadLonLatVec2 {
            x: -PI,
            y: self.lat_min,
        })
    }

    fn gcs_to_abs(&self, pos: &RadLonLatVec2) -> Vec2 {
        Vec2::new(pos.x, (PI / 4.0 + pos.y / 2.0).tan().ln())
    }

    fn abs_to_gcs(&self, pos: &Vec2) -> RadLonLatVec2 {
        RadLonLatVec2 {
            x: pos.x,
            y: pos.y.sinh().atan(),
        }
    }
}
