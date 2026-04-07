use crate::utils::glam_ext::bounding::{AxisAlignedBoundingBox2D, DAabb2};
use bevy::prelude::Reflect;
use glam::{DVec2, USizeVec2, dvec2};
use smallvec::SmallVec;

pub type TileKey = SmallVec<[SubDivisionKey; 10]>;

pub struct SubDivision2d {
    pub area: DAabb2,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, Reflect)]
pub enum SubDivisionKey {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

#[derive(Debug, Clone)]
pub struct Tile2d {
    pub key: TileKey,
    pub area: DAabb2,
}

impl SubDivision2d {
    pub fn area_size_for_min_depth_for_tile_count(
        &self,
        depth: usize,
        min_count: USizeVec2,
    ) -> DVec2 {
        (self.area.size() * dvec2(min_count.x as f64, min_count.y as f64)) / 2f64.powf(depth as f64)
    }

    pub fn min_depth_for_tile_count(&self, area_size: DVec2, min_count: USizeVec2) -> usize {
        if area_size.x == 0.0 || area_size.y == 0.0 {
            return 0;
        }

        let x_depth = (min_count.x as f64 * self.area.size().x / area_size.x)
            .max(1.0)
            .log2()
            .ceil() as usize;
        let y_depth = (min_count.y as f64 * self.area.size().y / area_size.y)
            .max(1.0)
            .log2()
            .ceil() as usize;

        x_depth.max(y_depth).max(1)
    }

    pub fn tile_path(&self, pos: DVec2, depth: usize) -> TileKey {
        let mut path = TileKey::with_capacity(depth);
        let mut rem = pos - self.area.min();

        for idx in 0..depth {
            let tile_size = self.area.size() / 2.0f64.powf(idx as f64);
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

    pub fn tile_bbox(&self, key: &TileKey) -> DAabb2 {
        let mut bb_min = self.area.min();

        let mut rem_tile_size = self.area.size();
        for sub_key in key.iter() {
            bb_min += rem_tile_size
                * match sub_key {
                    SubDivisionKey::TopRight => DVec2::new(0.5, 0.5),
                    SubDivisionKey::TopLeft => DVec2::new(0.0, 0.5),
                    SubDivisionKey::BottomRight => DVec2::new(0.5, 0.0),
                    SubDivisionKey::BottomLeft => DVec2::new(0.0, 0.0),
                };
            rem_tile_size /= 2.0;
        }

        DAabb2::new(bb_min, bb_min + rem_tile_size)
    }

    pub fn tile_covering(&self, area: DAabb2, depth: usize) -> impl Iterator<Item = Tile2d> {
        let tile_size = self.area.size() / 2.0f64.powf(depth as f64);

        let start =
            self.area.min() + tile_size * ((area.min() - self.area.min()) / tile_size).floor();
        let end = self.area.min() + tile_size * ((area.max() - self.area.min()) / tile_size).ceil();
        let count = ((end - start) / tile_size).ceil().as_usizevec2();

        (0..count.x).flat_map(move |x| {
            (0..count.y).filter_map(move |y| {
                let tile_area = DAabb2::from_center(
                    start + tile_size * DVec2::new(x as f64 + 0.5, y as f64 + 0.5),
                    tile_size,
                );

                if self.area.contains(tile_area.size()) {
                    let key = self.tile_path(tile_area.center(), depth);

                    Some(Tile2d {
                        key,
                        area: tile_area,
                    })
                } else {
                    None
                }
            })
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn test_tile_bbox() {
        let sub_div = SubDivision2d {
            area: DAabb2::new(dvec2(-10.0, 5.0), dvec2(10.0, 10.0)),
        };

        assert_eq!(
            sub_div.tile_bbox(&TileKey::from_iter([
                SubDivisionKey::TopRight,
                SubDivisionKey::BottomLeft,
            ])),
            DAabb2::new(dvec2(0.0, 7.5), dvec2(5.0, 8.75))
        );
    }

    #[test]
    fn test_tile_covering() {
        let sub_div = SubDivision2d {
            area: DAabb2::new(dvec2(-10.0, 5.0), dvec2(10.0, 10.0)),
        };

        assert_eq!(
            sub_div
                .tile_covering(DAabb2::new(dvec2(0.0, 7.5), dvec2(5.0, 8.75)), 2)
                .map(|tile| tile.key)
                .collect::<HashSet<_>>(),
            HashSet::from_iter([TileKey::from_iter([
                SubDivisionKey::TopRight,
                SubDivisionKey::BottomLeft,
            ])])
        );

        assert_eq!(
            sub_div
                .tile_covering(DAabb2::new(dvec2(0.0, 7.5), dvec2(5.0, 8.75)), 3)
                .map(|tile| tile.key)
                .collect::<HashSet<_>>(),
            HashSet::from_iter([
                TileKey::from_iter([
                    SubDivisionKey::TopRight,
                    SubDivisionKey::BottomLeft,
                    SubDivisionKey::BottomRight
                ]),
                TileKey::from_iter([
                    SubDivisionKey::TopRight,
                    SubDivisionKey::BottomLeft,
                    SubDivisionKey::BottomLeft
                ]),
                TileKey::from_iter([
                    SubDivisionKey::TopRight,
                    SubDivisionKey::BottomLeft,
                    SubDivisionKey::TopRight
                ]),
                TileKey::from_iter([
                    SubDivisionKey::TopRight,
                    SubDivisionKey::BottomLeft,
                    SubDivisionKey::TopLeft
                ]),
            ])
        );

        assert_eq!(
            sub_div
                .tile_covering(DAabb2::new(dvec2(0.0, 8.7), dvec2(5.0, 8.75)), 2)
                .map(|tile| tile.key)
                .collect::<HashSet<_>>(),
            HashSet::from_iter([TileKey::from_iter([
                SubDivisionKey::TopRight,
                SubDivisionKey::BottomLeft,
            ])])
        );
    }
}
