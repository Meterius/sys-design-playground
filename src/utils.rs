use bevy::math::Vec2;
use bevy::math::bounding::Aabb2d;

pub trait Aabb2dSized {
    fn size(&self) -> Vec2;
}

impl Aabb2dSized for Aabb2d {
    fn size(&self) -> Vec2 {
        self.max - self.min
    }
}

pub trait Aabb2dFromCorners {
    fn from_corners(a: Vec2, b: Vec2) -> Self;
}

impl Aabb2dFromCorners for Aabb2d {
    fn from_corners(a: Vec2, b: Vec2) -> Self {
        Self::new((a + b) / 2.0, (a.max(b) - a.min(b)) / 2.0)
    }
}
