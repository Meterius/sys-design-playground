use crate::glam_ext::bounding::{AxisAlignedBoundingBox2D, DAabb2};
use crate::image_utils::copy_from_sub_image;
use glam::{DVec2, UVec2, dvec2};

pub struct TiledImage<S> {
    pub source: S,
    pub tile_resolution: UVec2,
    pub tile_count: UVec2,
}

pub trait TiledImageSource {
    type Error;
    fn load_tile(&self, tile_index: UVec2) -> Result<image::RgbImage, Self::Error>;
}

pub struct TiledImageChunk {
    pub tile_index: UVec2,
    pub tile_area: (UVec2, UVec2),
    pub area: (UVec2, UVec2),
}

impl<S: TiledImageSource> TiledImage<S> {
    fn pos_to_tile_index(&self, pos: UVec2) -> UVec2 {
        pos / self.tile_resolution
    }

    fn area_chunks(&self, area: (UVec2, UVec2)) -> impl Iterator<Item = TiledImageChunk> {
        let (start, end) = (
            self.pos_to_tile_index(area.0),
            self.pos_to_tile_index(area.1),
        );

        (start.x..=end.x).flat_map(move |x| {
            (start.y..=end.y).map(move |y| {
                let tile_index = UVec2::new(x, y);

                let tile_area = (
                    area.0
                        .saturating_sub(self.tile_resolution * tile_index)
                        .min(self.tile_resolution - 1),
                    area.1
                        .saturating_sub(self.tile_resolution * tile_index)
                        .min(self.tile_resolution - 1),
                );

                TiledImageChunk {
                    tile_index,
                    tile_area,
                    area: (
                        tile_index * self.tile_resolution + tile_area.0,
                        tile_index * self.tile_resolution + tile_area.1,
                    ),
                }
            })
        })
    }

    pub fn load_sub_image(&self, area: (UVec2, UVec2)) -> Result<image::RgbImage, S::Error> {
        debug_assert!(area.0.cmple(area.1).all());
        debug_assert!(
            area.1
                .cmple(self.tile_resolution * self.tile_count - UVec2::ONE)
                .all()
        );

        let mut out = image::RgbImage::new(area.1.x - area.0.x + 1, area.1.y - area.0.y + 1);

        for chunk in self.area_chunks(area) {
            let tile_img = self.source.load_tile(chunk.tile_index)?;
            copy_from_sub_image(&tile_img, &mut out, chunk.tile_area, chunk.area.0 - area.0);
        }

        Ok(out)
    }
}

pub struct LinearTiledImage<S> {
    pub tiled_image: TiledImage<S>,
    pub bounds: DAabb2,
}

impl<S: TiledImageSource> LinearTiledImage<S> {
    fn total_resolution(&self) -> UVec2 {
        self.tiled_image.tile_resolution * self.tiled_image.tile_count
    }

    fn world_to_px_corner(&self, world: DVec2) -> DVec2 {
        (dvec2(0.0, 1.0) + dvec2(1.0, -1.0) * (world - self.bounds.min()) / self.bounds.size())
            * self.total_resolution().as_dvec2()
    }

    pub fn load_sub_image(
        &self,
        area: DAabb2,
        resolution: UVec2,
    ) -> Result<image::RgbImage, S::Error> {
        let total = self.total_resolution();

        // Corner-pixel range of the requested area in global source space.
        let px_bounds = DAabb2::from_corners(
            self.world_to_px_corner(area.min()),
            self.world_to_px_corner(area.max()),
        );

        // Inclusive pixel indices to load.  Bilinear at a position t reads pixels
        // floor(t) and floor(t)+1, so we load from floor(px_min) to floor(px_max)+1.
        let load_min = UVec2::new(
            px_bounds.min().x.floor().max(0.0) as u32,
            px_bounds.min().y.floor().max(0.0) as u32,
        );
        let load_max = UVec2::new(
            (px_bounds.max().x.floor() as u32 + 1).min(total.x - 1),
            (px_bounds.max().y.floor() as u32 + 1).min(total.y - 1),
        );

        let sub_img = self.tiled_image.load_sub_image((load_min, load_max))?;
        let sub_size = DVec2::new(sub_img.width() as f64, sub_img.height() as f64);

        let mut out = image::RgbImage::new(resolution.x, resolution.y);

        for oy in 0..resolution.y {
            for ox in 0..resolution.x {
                // World position of output pixel center.
                let world = area.min()
                    + area.size()
                        * DVec2::new(
                            (ox as f64 + 0.5) / resolution.x as f64,
                            ((resolution.y - oy - 1) as f64 + 0.5) / resolution.y as f64,
                        );

                // Corner-pixel coordinate in the sub-image space, normalised to [0, 1].
                let px_local = self.world_to_px_corner(world) - load_min.as_dvec2();
                let u = (px_local.x / sub_size.x).clamp(0.0, 1.0) as f32;
                let v = (px_local.y / sub_size.y).clamp(0.0, 1.0) as f32;

                if let Some(pixel) = image::imageops::sample_bilinear(&sub_img, u, v) {
                    out.put_pixel(ox, oy, pixel);
                }
            }
        }

        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use crate::glam_ext::bounding::{AxisAlignedBoundingBox2D, DAabb2};
    use crate::tiled_imaging::{LinearTiledImage, TiledImage, TiledImageSource};
    use anyhow::anyhow;
    use glam::{UVec2, dvec2, uvec2};
    use image::RgbImage;
    use std::error::Error;

    #[test]
    fn test_tiled_image_area() {
        struct Source {}

        impl TiledImageSource for Source {
            type Error = anyhow::Error;

            fn load_tile(&self, tile_index: UVec2) -> Result<RgbImage, Self::Error> {
                Ok(image::ImageReader::open(format!(
                    "test-data/tile_{}{}.tiff",
                    tile_index.x, tile_index.y
                ))?
                .decode()?
                .to_rgb8())
            }
        }

        let tiled_image = TiledImage {
            source: Source {},
            tile_resolution: uvec2(32, 16),
            tile_count: uvec2(2, 3),
        };

        let expected = image::ImageReader::open("test-data/tiling_res.tiff")
            .unwrap()
            .decode()
            .unwrap()
            .to_rgb8();
        assert_eq!(
            tiled_image
                .load_sub_image((uvec2(14, 5), uvec2(53, 16)))
                .unwrap(),
            expected
        );
    }

    #[test]
    fn test_linear_tiled_image_area() {
        struct Source {}

        impl TiledImageSource for Source {
            type Error = anyhow::Error;

            fn load_tile(&self, tile_index: UVec2) -> Result<RgbImage, Self::Error> {
                Ok(image::ImageReader::open(format!(
                    "test-data/tile_{}{}.tiff",
                    tile_index.x, tile_index.y
                ))?
                .decode()?
                .to_rgb8())
            }
        }

        let tiled_image = LinearTiledImage {
            tiled_image: TiledImage {
                source: Source {},
                tile_resolution: uvec2(32, 16),
                tile_count: uvec2(2, 3),
            },
            bounds: DAabb2::new(dvec2(-180.0, -90.0), dvec2(180.0, 90.0)),
        };

        let res = tiled_image
            .load_sub_image(
                DAabb2::new(dvec2(-180.0, -90.0), dvec2(0.0, 0.0)),
                uvec2(128, 128),
            )
            .unwrap();
        res.save("test-data/linear_tiling_res.tiff").unwrap();
    }
}
