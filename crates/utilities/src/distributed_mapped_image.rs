use crate::glam_ext::bounding::{AxisAlignedBoundingBox2D, DAabb2};
use anyhow::Context;
use glam::{DVec2, IVec2, UVec2, dvec2};
use rayon::iter::IntoParallelIterator;
use rayon::iter::ParallelIterator;
use rstar::{AABB, RTree, RTreeObject};
use std::path::{Path, PathBuf};

#[derive(serde::Serialize, serde::Deserialize)]
pub struct TileMeta {
    pub bounds: DAabb2,
}

struct SourceEntry {
    bounds: DAabb2,
    path: PathBuf,
}

impl RTreeObject for SourceEntry {
    type Envelope = AABB<[f64; 2]>;

    fn envelope(&self) -> Self::Envelope {
        AABB::from_corners(
            [self.bounds.min().x, self.bounds.min().y],
            [self.bounds.max().x, self.bounds.max().y],
        )
    }
}

/// A set of geo-referenced images, each covering an arbitrary `DAabb2` region,
/// that can be composited into a single output image for any requested area and
/// resolution.
///
/// Sources are indexed in an R-tree so that `load_sub_image` only considers
/// those that actually intersect the requested area.
///
/// This generalises [`crate::tiled_imaging::LinearTiledImage`], which is the
/// special case where every source has the same size and the sources tile a
/// regular grid.
pub struct DistributedMappedImage {
    tree: RTree<SourceEntry>,
}

impl DistributedMappedImage {
    /// Build from an iterator of `(bounds, image_path)` pairs.
    pub fn new(sources: impl IntoIterator<Item = (DAabb2, PathBuf)>) -> Self {
        let entries: Vec<SourceEntry> = sources
            .into_iter()
            .map(|(bounds, path)| SourceEntry { bounds, path })
            .collect();
        Self {
            tree: RTree::bulk_load(entries),
        }
    }

    /// Scan `dir` for tile pairs and build a `DistributedMappedImage`.
    ///
    /// A tile pair is a file `<stem>` and a companion `<stem>.meta.json` in
    /// the same directory.  The JSON must deserialise into `TileMeta`:
    ///
    /// ```json
    /// { "bounds": { "min": [x_min, y_min], "max": [x_max, y_max] } }
    /// ```
    ///
    /// Every `.meta.json` file found in `dir` (non-recursively) is treated as
    /// one tile; the corresponding image path is the name with `.meta.json`
    /// stripped.
    pub fn from_directory(dir: impl AsRef<Path>) -> anyhow::Result<Self> {
        let dir = dir.as_ref();

        let sources = std::fs::read_dir(dir)
            .with_context(|| format!("reading directory {dir:?}"))?
            .map(|entry| entry.with_context(|| format!("reading entry in {dir:?}")))
            .collect::<anyhow::Result<Vec<_>>>()?
            .into_par_iter()
            .filter(|e| e.file_name().to_string_lossy().ends_with(".meta.json"))
            .map(|e| {
                let meta_path = e.path();
                let image_path = meta_path.with_extension("").with_extension(""); // strip .meta.json

                let meta: TileMeta = serde_json::from_reader(
                    std::fs::File::open(&meta_path)
                        .with_context(|| format!("opening {meta_path:?}"))?,
                )
                .with_context(|| format!("parsing {meta_path:?}"))?;

                Ok((meta.bounds, image_path))
            })
            .collect::<anyhow::Result<Vec<_>>>()?;

        Ok(Self::new(sources))
    }

    /// Render `area` at the given pixel `resolution`.
    ///
    /// Only the source images that overlap `area` are loaded from disk.  Each
    /// output pixel is filled by the first source (in R-tree iteration order)
    /// whose bounds contain the pixel's world position, sampled with bilinear
    /// interpolation.  Pixels not covered by any source are left black.
    pub fn load_sub_image(
        &self,
        area: DAabb2,
        resolution: UVec2,
    ) -> anyhow::Result<Option<image::RgbImage>> {
        let query = AABB::from_corners([area.min().x, area.min().y], [area.max().x, area.max().y]);

        // Load only the sources that intersect the requested area.
        let loaded: Vec<(DAabb2, image::RgbImage)> = self
            .tree
            .locate_in_envelope_intersecting(&query)
            .filter(|entry| {
                entry
                    .bounds
                    .intersection(area)
                    .is_some_and(|overlap| overlap.size().min_element() > 0.0)
            })
            .collect::<Vec<_>>()
            .into_par_iter()
            .map(|entry| {
                let img = image::ImageReader::open(&entry.path)
                    .with_context(|| format!("opening {:?}", entry.path))?
                    .decode()
                    .with_context(|| format!("decoding {:?}", entry.path))?
                    .to_rgb8();
                Ok((entry.bounds, img))
            })
            .collect::<anyhow::Result<_>>()?;

        if loaded.is_empty() {
            return Ok(None);
        }

        let mut out = image::RgbImage::new(resolution.x, resolution.y);

        for (bounds, img) in &loaded {
            if let Some(overlap) = bounds.intersection(area) {
                let overlap_rel = DAabb2::new(
                    (overlap.min() - area.min()) / area.size(),
                    (overlap.max() - area.min()) / area.size(),
                );

                let to_pixel = |rel_pos: DVec2| {
                    dvec2(0.0, resolution.y as f64)
                        + dvec2(1.0, -1.0) * (rel_pos * resolution.as_dvec2() + dvec2(-0.5, 0.5))
                };

                let start = to_pixel(overlap_rel.min());
                let end = to_pixel(overlap_rel.max());
                let (start, end) = (dvec2(start.x, end.y), dvec2(end.x, start.y));
                let start = start.ceil().as_ivec2();
                let end = end.floor().as_ivec2();

                if 0 <= end.y
                    && start.y < resolution.y as i32
                    && 0 <= end.x
                    && start.x < resolution.x as i32
                    && start.x <= end.x
                    && start.y <= end.y
                {
                    let start = start
                        .clamp(IVec2::ZERO, resolution.as_ivec2() - IVec2::ONE)
                        .as_uvec2();
                    let end = end
                        .clamp(IVec2::ZERO, resolution.as_ivec2() - IVec2::ONE)
                        .as_uvec2();

                    for oy in start.y..=end.y {
                        for ox in start.x..=end.x {
                            // World position of this output pixel's center.
                            // World Y increases upward; image row 0 is the top, so output
                            // row `oy` corresponds to world fraction
                            // `(resolution.y - oy - 1 + 0.5) / resolution.y`.
                            let world = area.min()
                                + area.size()
                                    * DVec2::new(
                                        (ox as f64 + 0.5) / resolution.x as f64,
                                        ((resolution.y - oy - 1) as f64 + 0.5)
                                            / resolution.y as f64,
                                    );

                            let img_res = dvec2(img.width() as f64, img.height() as f64);

                            // Corner-pixel coordinate within the source image.
                            // Mirrors `LinearTiledImage::world_to_px_corner`: world Y
                            // max maps to pixel row 0 (top), world Y min maps to the
                            // bottom row.
                            let px = (dvec2(0.0, 1.0)
                                + dvec2(1.0, -1.0) * (world - bounds.min()) / bounds.size())
                                * img_res;

                            let u = (px.x / img_res.x).clamp(0.0, 1.0) as f32;
                            let v = (px.y / img_res.y).clamp(0.0, 1.0) as f32;

                            if let Some(pixel) = image::imageops::sample_bilinear(img, u, v) {
                                out.put_pixel(ox, oy, pixel);
                            }
                        }
                    }
                }
            }
        }

        Ok(Some(out))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::uvec2;

    #[test]
    fn test_load_sub_image() {
        let im =
            DistributedMappedImage::from_directory("test-data/distributed_image/dir-a").unwrap();

        for (bounds, name) in [
            (
                DAabb2::new(dvec2(-128.0, -128.0), dvec2(128.0, 128.0)),
                "full",
            ),
            (DAabb2::new(dvec2(-128.0, 0.0), dvec2(0.0, 128.0)), "tl"),
            (DAabb2::new(dvec2(0.0, 0.0), dvec2(128.0, 128.0)), "tr"),
            (DAabb2::new(dvec2(-128.0, -128.0), dvec2(0.0, 0.0)), "bl"),
            (DAabb2::new(dvec2(0.0, -128.0), dvec2(128.0, 0.0)), "br"),
        ]
        .into_iter()
        {
            let res = im.load_sub_image(bounds, uvec2(64, 64)).unwrap().unwrap();
            res.save(format!("test-data/distributed_image/dir-a_{name}.png"))
                .unwrap();
        }

        let im =
            DistributedMappedImage::from_directory("test-data/distributed_image/dir-b").unwrap();

        for (bounds, name) in [
            (
                DAabb2::new(dvec2(-128.0, -128.0), dvec2(128.0, 128.0)),
                "full",
            ),
            (DAabb2::new(dvec2(-128.0, 0.0), dvec2(0.0, 128.0)), "tl"),
            (DAabb2::new(dvec2(0.0, 0.0), dvec2(128.0, 128.0)), "tr"),
            (DAabb2::new(dvec2(-128.0, -128.0), dvec2(0.0, 0.0)), "bl"),
            (DAabb2::new(dvec2(0.0, -128.0), dvec2(128.0, 0.0)), "br"),
            (DAabb2::new(dvec2(-64.0, -120.0), dvec2(72.0, 30.0)), "rnd"),
        ]
        .into_iter()
        {
            let res = im.load_sub_image(bounds, uvec2(32, 32)).unwrap().unwrap();
            res.save(format!("test-data/distributed_image/dir-b_{name}.png"))
                .unwrap();
        }
    }
}
