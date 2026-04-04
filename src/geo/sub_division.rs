use bevy::math::USizeVec2;
use bevy::prelude::Vec2;
use smallvec::SmallVec;

pub type TileKey = SmallVec<[SubDivisionKey; 10]>;

pub struct SubDivision2d {
    pub bb_min: Vec2,
    pub bb_max: Vec2,
    pub bb_size: Vec2,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum SubDivisionKey {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

#[derive(Debug, Clone)]
pub struct Tile2d {
    pub key: TileKey,
    pub bb_min: Vec2,
    pub bb_max: Vec2,
    pub bb_size: Vec2,
}

impl SubDivision2d {
    pub fn from_corners(a: Vec2, b: Vec2) -> Self {
        Self {
            bb_min: a.min(b),
            bb_max: a.max(b),
            bb_size: a.max(b) - a.min(b),
        }
    }

    pub fn area_size_for_min_depth_for_tile_count(
        &self,
        depth: usize,
        min_count: USizeVec2,
    ) -> Vec2 {
        (self.bb_size * Vec2::new(min_count.x as f32, min_count.y as f32)) / 2f32.powf(depth as f32)
    }

    pub fn min_depth_for_tile_count(&self, area_size: Vec2, min_count: USizeVec2) -> usize {
        if area_size.x == 0.0 || area_size.y == 0.0 {
            return 0;
        }

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

    pub fn tile_path(&self, pos: Vec2, depth: usize) -> TileKey {
        let mut path = TileKey::with_capacity(depth);
        let mut rem = pos - self.bb_min;

        for idx in 0..depth {
            let tile_size = self.bb_size / 2.0f32.powf(idx as f32);
            let hor_bucket = rem.x >= tile_size.x / 2.0;
            let ver_bucket = rem.y >= tile_size.y / 2.0;

            rem.x %= tile_size.x / 2.0;
            rem.y %= tile_size.y / 2.0;

            path.push(match (hor_bucket, ver_bucket) {
                (true, true) => SubDivisionKey::TopRight,
                (true, false) => SubDivisionKey::BottomRight,
                (false, true) => SubDivisionKey::TopLeft,
                (false, false) => SubDivisionKey::BottomLeft,
            });
        }

        path
    }

    pub fn tile_bbox(&self, key: &TileKey) -> (Vec2, Vec2) {
        let mut bb_min = self.bb_min;

        let mut rem_tile_size = self.bb_size;
        for sub_key in key.iter() {
            bb_min += rem_tile_size
                * match sub_key {
                    SubDivisionKey::TopRight => Vec2::new(0.5, 0.5),
                    SubDivisionKey::TopLeft => Vec2::new(0.0, 0.5),
                    SubDivisionKey::BottomRight => Vec2::new(0.5, 0.0),
                    SubDivisionKey::BottomLeft => Vec2::new(0.0, 0.0),
                };
            rem_tile_size /= 2.0;
        }

        (bb_min, bb_min + rem_tile_size)
    }

    pub fn tile_covering(&self, area: (Vec2, Vec2), depth: usize) -> impl Iterator<Item = Tile2d> {
        let area_bb_min = area.0.min(area.1).max(self.bb_min);
        let area_bb_max = area.0.max(area.1).min(self.bb_max);

        let tile_size = self.bb_size / 2.0f32.powf(depth as f32);

        let start = self.bb_min + tile_size * ((area_bb_min - self.bb_min) / tile_size).floor();
        let end = self.bb_min + tile_size * ((area_bb_max - self.bb_min) / tile_size).ceil();
        let count = ((end - start) / tile_size).ceil().as_usizevec2();

        (0..count.x).flat_map(move |x| {
            (0..count.y).map(move |y| {
                let tile_bb_min = start + tile_size * Vec2::new(x as f32, y as f32);
                let tile_bb_max = tile_bb_min + tile_size;
                Tile2d {
                    key: self.tile_path((tile_bb_max + tile_bb_min) / 2.0, depth),
                    bb_max: tile_bb_max,
                    bb_min: tile_bb_min,
                    bb_size: tile_size,
                }
            })
        })
    }
}
