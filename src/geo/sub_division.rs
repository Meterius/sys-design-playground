use bevy::math::USizeVec2;
use bevy::prelude::Vec2;
use itertools::Itertools;
use serde::de::value::UsizeDeserializer;

pub struct SubDivision2d {
    pub bb_min: Vec2,
    pub bb_max: Vec2,
    pub bb_size: Vec2,
}

#[derive(Debug, Clone)]
pub struct Tile2d {
    pub bb_min: Vec2,
    pub bb_max: Vec2,
    pub bb_size: Vec2,
}

impl Tile2d {
    pub fn subdivided(&self) -> impl Iterator<Item = Tile2d> {
        [0.0, 0.5].into_iter().cartesian_product([0.0, 0.5].into_iter()).map(|(a, b)| Tile2d {
            bb_min: self.bb_min + (self.bb_max - self.bb_min) * Vec2::new(a, b),
            bb_max: self.bb_max + (self.bb_max - self.bb_min) * Vec2::new(a + 0.5, b + 0.5),
            bb_size: self.bb_size / 2.0,
        })
    }
}

impl SubDivision2d {
    pub fn from_corners(a: Vec2, b: Vec2) -> Self {
        Self {
            bb_min: a.min(b),
            bb_max: a.max(b),
            bb_size: a.max(b) - a.min(b),
        }
    }

    pub fn min_depth_for_tile_count(&self, area: (Vec2, Vec2), min_count: USizeVec2) -> usize {
        let area_size = area.0.max(area.1) - area.0.min(area.1);

        let x_depth = (min_count.x as f32 * self.bb_size.x / area_size.x)
            .max(1.0)
            .log2()
            .ceil() as usize;
        let y_depth = (min_count.y as f32 * self.bb_size.y / area_size.y)
            .max(1.0)
            .log2()
            .ceil() as usize;

       x_depth.max(y_depth).max(1)
    }

    pub fn tile_covering(
        &self, area: (Vec2, Vec2), depth: usize
    ) -> impl Iterator<Item = Tile2d> {
        let area_bb_min = area.0.min(area.1);
        let area_bb_max = area.0.max(area.1);

        let tile_size = self.bb_size / 2.0f32.powf(depth as f32);

        let start = self.bb_min + tile_size * ((area_bb_min - self.bb_min) / tile_size).floor();
        let end = self.bb_min + tile_size * ((area_bb_max - self.bb_min) / tile_size).ceil();
        let count = ((end - start) / tile_size).ceil().as_usizevec2();

        (0..count.x).flat_map(move |x| {
            (0..count.y).map(move |y| {
                let tile_bb_min = start + tile_size * Vec2::new(x as f32, y as f32);
                let tile_bb_max = tile_bb_min + tile_size;
                Tile2d {
                    bb_max: tile_bb_max,
                    bb_min: tile_bb_min,
                    bb_size: tile_size,
                }
            })
        })
    }
}
