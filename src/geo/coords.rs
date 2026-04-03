use bevy::prelude::Vec2;
use std::f32::consts::PI;

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

#[derive(Debug, Clone)]
pub struct BoundedMercatorProjection {
    pub lat_max: f32,
    pub lat_min: f32,
    pub scale: f32,
}

impl BoundedMercatorProjection {
    pub fn gcs_pos(&self) -> RadLonLatVec2 {
        RadLonLatVec2 { x: 0.0, y: 0.0 }
    }

    pub fn gcs_size(&self) -> Vec2 {
        Vec2::new(2.0 * PI, PI)
    }

    pub fn abs_pos(&self) -> Vec2 {
        self.gcs_to_abs(&RadLonLatVec2 { x: 0.0, y: 0.0 })
    }

    pub fn abs_size(&self) -> Vec2 {
        self.gcs_to_abs(&RadLonLatVec2 {
            x: PI,
            y: self.lat_max,
        }) - self.gcs_to_abs(&RadLonLatVec2 {
            x: -PI,
            y: self.lat_min,
        })
    }

    pub fn gcs_to_abs(&self, pos: &RadLonLatVec2) -> Vec2 {
        Vec2::new(
            self.scale * pos.x,
            self.scale * (PI / 4.0 + pos.y / 2.0).tan().ln(),
        )
    }

    pub fn abs_to_gcs(&self, pos: &Vec2) -> RadLonLatVec2 {
        RadLonLatVec2 {
            x: pos.x / self.scale,
            y: (pos.y / self.scale).sinh().atan(),
        }
    }
}
