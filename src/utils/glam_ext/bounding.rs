use bevy::prelude::Reflect;
use glam::{DVec2, Vec2};
use std::ops::{Add, Div, Sub};

#[derive(Debug, Clone, Copy, PartialEq, Reflect)]
pub struct Aabb2 {
    min: Vec2,
    max: Vec2,
}

#[derive(Debug, Clone, Copy, PartialEq, Reflect)]
pub struct DAabb2 {
    min: DVec2,
    max: DVec2,
}

trait Scalar {
    fn two() -> Self;

    fn zero() -> Self;
}

impl Scalar for f32 {
    fn two() -> Self {
        2.0
    }

    fn zero() -> Self {
        0.0
    }
}

impl Scalar for f64 {
    fn two() -> Self {
        2.0
    }

    fn zero() -> Self {
        0.0
    }
}

trait Vector<V> {
    fn min(self, other: Self) -> Self;

    fn max(self, other: Self) -> Self;

    fn from_array(arr: [V; 2]) -> Self;

    fn to_array(&self) -> [V; 2];
}

impl Vector<f32> for Vec2 {
    fn min(self, other: Self) -> Self {
        Vec2::min(self, other)
    }

    fn max(self, other: Self) -> Self {
        Vec2::max(self, other)
    }

    fn from_array(arr: [f32; 2]) -> Self {
        Self::from_array(arr)
    }

    fn to_array(&self) -> [f32; 2] {
        self.to_array()
    }
}

impl Vector<f64> for DVec2 {
    fn min(self, other: Self) -> Self {
        DVec2::min(self, other)
    }

    fn max(self, other: Self) -> Self {
        DVec2::max(self, other)
    }

    fn from_array(arr: [f64; 2]) -> Self {
        Self::from_array(arr)
    }

    fn to_array(&self) -> [f64; 2] {
        self.to_array()
    }
}

impl AxisAlignedBoundingBox2D<Vec2, f32> for Aabb2 {
    fn min(&self) -> Vec2 {
        self.min
    }

    fn max(&self) -> Vec2 {
        self.max
    }

    fn new(min: Vec2, max: Vec2) -> Self {
        debug_assert!(min.x <= max.x && min.y <= max.y);
        Self { min, max }
    }
}

impl AxisAlignedBoundingBox2D<DVec2, f64> for DAabb2 {
    fn min(&self) -> DVec2 {
        self.min
    }

    fn max(&self) -> DVec2 {
        self.max
    }

    fn new(min: DVec2, max: DVec2) -> Self {
        debug_assert!(min.x <= max.x && min.y <= max.y);
        Self { min, max }
    }
}

pub trait AxisAlignedBoundingBox2D<T, V>
where
    Self: Sized,
    T: Copy + Add<Output = T> + Div<V, Output = T> + Sub<Output = T> + Vector<V>,
    V: Scalar + PartialOrd + Copy,
{
    fn min(&self) -> T;

    fn max(&self) -> T;

    fn new(min: T, max: T) -> Self;
    
    fn from_corners(a: T, b: T) -> Self {
        Self::new(a.min(b), b.max(a))
    }

    fn from_center(center: T, size: T) -> Self {
        debug_assert!(size.to_array().into_iter().all(|v| v >= V::zero()));
        Self::new(center - size / V::two(), center + size / V::two())
    }

    fn center(&self) -> T {
        (self.min() + self.max()) / V::two()
    }

    fn size(&self) -> T {
        self.max() - self.min()
    }

    fn intersection(&self, other: Self) -> Option<Self> {
        let min = self.min().max(other.min());
        let max = self.max().min(other.max());

        let [min_x, min_y] = min.to_array();
        let [max_x, max_y] = max.to_array();

        if min_x <= max_x && min_y <= max_y {
            Some(Self::new(min, max))
        } else {
            None
        }
    }

    fn expand(&self, border: T) -> Self {
        Self::new(self.min() - border, self.max() + border)
    }

    fn contains(&self, pos: T) -> bool {
        let [x, y] = pos.to_array();
        let [min_x, min_y] = self.min().to_array();
        let [max_x, max_y] = self.max().to_array();
        min_x <= x && x <= max_x && min_y <= y && y <= max_y
    }

    fn closest_point(&self, pos: T) -> T {
        pos.max(self.min()).min(self.max())
    }

    fn corners(&self) -> impl Iterator<Item = T> {
        let [min_x, min_y] = self.min().to_array();
        let [max_x, max_y] = self.max().to_array();

        [
            T::from_array([min_x, min_y]),
            T::from_array([min_x, max_y]),
            T::from_array([max_x, max_y]),
            T::from_array([max_x, min_y]),
        ]
        .into_iter()
    }
}
