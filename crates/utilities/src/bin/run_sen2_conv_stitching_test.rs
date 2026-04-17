use anyhow::{Context, anyhow, bail};
use glam::uvec2;
use rayon::iter::IndexedParallelIterator;
use rayon::iter::ParallelIterator;
use rayon::prelude::ParallelSliceMut;
use std::fs::File;
use std::path::{Path, PathBuf};
use utilities::distributed_mapped_image::{DistributedMappedImage, TileMeta};
use utilities::glam_ext::bounding::{AxisAlignedBoundingBox2D, DAabb2};
use utilities::sen2::{UserData, convert_sen2_img_to_epsg4326};

struct TiffBand {
    width: u32,
    height: u32,
    data: Vec<i16>,
}

fn read_tiff_band(path: impl AsRef<Path>) -> anyhow::Result<TiffBand> {
    let path = path.as_ref();
    let mut decoder =
        tiff::decoder::Decoder::new(File::open(path).with_context(|| format!("opening {path:?}"))?)
            .with_context(|| format!("creating TIFF decoder for {path:?}"))?;
    let (width, height) = decoder
        .dimensions()
        .with_context(|| format!("reading dimensions of {path:?}"))?;
    let data = match decoder
        .read_image()
        .with_context(|| format!("decoding {path:?}"))?
    {
        tiff::decoder::DecodingResult::I16(v) => v,
        tiff::decoder::DecodingResult::U16(v) => v.into_iter().map(|x| x as i16).collect(),
        other => bail!("Unexpected TIFF sample format in {path:?}: {other:?}"),
    };
    Ok(TiffBand {
        width,
        height,
        data,
    })
}

/// Read B04 (R), B03 (G), B02 (B) from `tile_dir` and merge into a single
/// RGB8 image. Applies the standard Sentinel-2 normalization: `2.5 * dn / 10000`.
fn merge_bands(tile_dir: &Path) -> anyhow::Result<image::RgbaImage> {
    let ((r, g), b) = rayon::join(
        || {
            rayon::join(
                || read_tiff_band(tile_dir.join("B04.tif")),
                || read_tiff_band(tile_dir.join("B03.tif")),
            )
        },
        || read_tiff_band(tile_dir.join("B02.tif")),
    );
    let (r, g, b) = (r?, g?, b?);

    anyhow::ensure!(
        r.width == g.width && g.width == b.width && r.height == g.height && g.height == b.height,
        "Band dimension mismatch in {tile_dir:?}: \
         B04={}×{}, B03={}×{}, B02={}×{}",
        r.width,
        r.height,
        g.width,
        g.height,
        b.width,
        b.height,
    );

    let mut raw = vec![0u8; (r.width * r.height) as usize * 4];
    raw.par_chunks_mut(4).enumerate().for_each(|(idx, pixel)| {
        let invalid = r.data[idx] == -32768 || g.data[idx] == -32768 || b.data[idx] == -32768;

        if !invalid {
            pixel[0] = (255.0 * 2.5 * r.data[idx] as f32 / 10000.0).clamp(0.0, 255.0) as u8;
            pixel[1] = (255.0 * 2.5 * g.data[idx] as f32 / 10000.0).clamp(0.0, 255.0) as u8;
            pixel[2] = (255.0 * 2.5 * b.data[idx] as f32 / 10000.0).clamp(0.0, 255.0) as u8;
            pixel[3] = 255;
        }
    });

    image::RgbaImage::from_raw(r.width, r.height, raw)
        .ok_or_else(|| anyhow!("Failed to construct RgbImage from raw buffer"))
}

fn main() {
    let src1_dir = PathBuf::from(
        "crates/utilities/test-data/sen2/conv-stitching/Sentinel-2_mosaic_2025_Q3_32UNE_0_0",
    );
    let src2_dir = PathBuf::from(
        "crates/utilities/test-data/sen2/conv-stitching/Sentinel-2_mosaic_2025_Q3_32UNF_0_0",
    );

    let out_dir = PathBuf::from("crates/utilities/test-data/sen2/conv-stitching/out");
    let out_base_dir = out_dir.join("base");

    let mut gcs_bounds: Option<DAabb2> = None;

    for src_dir in [&src1_dir, &src2_dir].iter() {
        let src_name = src_dir.file_name().unwrap().to_str().unwrap();
        println!("Processing {}", src_name);
        let src = merge_bands(&src_dir).unwrap();
        src.save(out_base_dir.join(src_name).with_extension("raw.png"))
            .unwrap();
        let ud: UserData =
            serde_json::from_reader(File::open(src_dir.join("userdata.json")).unwrap()).unwrap();

        let (out, bounds) = convert_sen2_img_to_epsg4326(&src, &ud).unwrap();
        out.save(out_base_dir.join(src_name).with_extension("png"))
            .unwrap();

        if let Some(prev_bounds) = gcs_bounds {
            gcs_bounds = Some(DAabb2::new(
                prev_bounds.min().min(bounds.min()),
                prev_bounds.max().max(bounds.max()),
            ));
        } else {
            gcs_bounds = Some(bounds);
        }

        serde_json::to_writer_pretty(
            File::create(out_base_dir.join(src_name).with_extension("png.meta.json")).unwrap(),
            &TileMeta { bounds },
        )
        .unwrap();
    }

    let gcs_bounds = gcs_bounds.unwrap();

    let dist_image = DistributedMappedImage::from_directory(out_base_dir).unwrap();
    let combined_width = 8192;
    let combined = dist_image
        .load_sub_image(
            gcs_bounds,
            uvec2(
                combined_width,
                (combined_width as f64 * 2.0 * gcs_bounds.size().y / gcs_bounds.size().x) as u32,
            ),
        )
        .unwrap()
        .unwrap();

    combined.save(out_dir.join("combined.png")).unwrap();
}
