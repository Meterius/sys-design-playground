use glam::{IVec2, dvec2, ivec2, usizevec2, UVec2};
use image::codecs::jpeg::JpegEncoder;
use image::imageops::crop_imm;
use image::{EncodableLayout, ImageEncoder, RgbImage};
use itertools::Itertools;
use rayon::prelude::*;
use std::fs::File;
use std::io::BufWriter;
use std::path::{Path, PathBuf};
use std::time::Instant;
use bevy::pbr::Falloff::Linear;
use tiff::decoder::ChunkType::Tile;
use utilities::glam_ext::bounding::{AxisAlignedBoundingBox2D, DAabb2};
use utilities::glam_ext::sub_division::{SubDivision2d, SubDivisionKey, TileKey};
use utilities::tiled_imaging::{LinearTiledImage, TiledImage, TiledImageSource};

pub const SOURCE_RES: usize = 10000;
pub const SOURCE_DIV: usize = 4;
pub const SUB_DIV_RES: usize = 1024;

pub const SOURCE_DIR: &str = "./datasets/sat/Sentinel-2_mosaic_2025_Q4/";
pub const OUT_DIR: &str = "./datasets/sat/Sentinel-2_mosaic_2025_Q4_conv/";

fn tile_name_to_bounds(tile_name: &str) -> Option<(IVec2, IVec2)> {
    let re = regex::Regex::new(r"([NS])(\d+)([EW])(\d+)").ok()?;
    let caps = re.captures(tile_name)?;

    let lat_val: f64 = caps[2].parse().ok()?;
    let lon_val: f64 = caps[4].parse().ok()?;

    let min_lat = if &caps[1] == "S" { -lat_val } else { lat_val };
    let max_lat = min_lat + 36.0;

    let min_lon = if &caps[3] == "E" { lon_val } else { -lon_val };
    let max_lon = min_lon + 36.0;

    assert_eq!(min_lat.round(), min_lat);
    assert_eq!(max_lat.round(), max_lat);
    assert_eq!(min_lon.round(), min_lon);
    assert_eq!(max_lon.round(), max_lon);

    Some((
        ivec2(min_lon as i32, min_lat as i32),
        ivec2(max_lon as i32, max_lat as i32),
    ))
}

struct TiffBand {
    width: u32,
    height: u32,
    data: Vec<i16>,
}

fn read_tiff_band(path: impl AsRef<Path>) -> TiffBand {
    let file = File::open(path).unwrap();
    let mut decoder = tiff::decoder::Decoder::new(file).unwrap();
    let (width, height) = decoder.dimensions().unwrap();
    let data = match decoder.read_image().unwrap() {
        tiff::decoder::DecodingResult::I16(v) => v,
        tiff::decoder::DecodingResult::U16(v) => v.into_iter().map(|x| x as i16).collect(),
        other => panic!("Unexpected TIFF sample format: {:?}", other),
    };
    TiffBand {
        width,
        height,
        data,
    }
}

fn base_tiles_out_dir(size: IVec2) -> PathBuf {
    PathBuf::from(OUT_DIR).join(format!("{}_{}", size.x, size.y))
}

fn base_tiles_out_file_path(bounds: (IVec2, IVec2)) -> PathBuf {
    let out_name = format!(
        "{}_{}_{}_{}",
        bounds.0.x, bounds.0.y, bounds.1.x, bounds.1.y
    );
    base_tiles_out_dir(bounds.1 - bounds.0)
        .join(out_name)
        .with_added_extension("tiff")
}

#[derive(Clone)]
struct BaseTileSource {
    pub tiles_dir: PathBuf,
    pub gcs_size: IVec2,
}

impl TiledImageSource for BaseTileSource {
    type Error = image::ImageError;

    fn load_tile(&self, tile_index: UVec2) -> Result<RgbImage, Self::Error> {
        let count = ivec2(360, 180) / self.gcs_size;
        let start = ivec2(-180, -90) + self.gcs_size * (ivec2(0, count.y - 1) + ivec2(1, -1) * tile_index.as_ivec2());
        let end = start + self.gcs_size;

        let tile_path = self.tiles_dir.join(format!("{}_{}_{}_{}.tiff", start.x, start.y, end.x, end.y));
        Ok(image::open(tile_path)?.to_rgb8())
    }
}

fn process_base_tile(tile_path: &Path, bounds: (IVec2, IVec2)) {
    let div_size = (SOURCE_RES / SOURCE_DIV, SOURCE_RES / SOURCE_DIV);
    let div_bound_size = (bounds.1 - bounds.0) / (SOURCE_DIV as i32);

    let ((r, g), b) = rayon::join(
        || {
            rayon::join(
                || read_tiff_band(tile_path.join("B04.tif")),
                || read_tiff_band(tile_path.join("B03.tif")),
            )
        },
        || read_tiff_band(tile_path.join("B02.tif")),
    );

    assert_eq!(r.width, SOURCE_RES as u32);
    assert_eq!(r.height, SOURCE_RES as u32);

    let pixel_count = (r.width * r.height) as usize;
    let mut raw = vec![0u8; pixel_count * 3];

    raw.par_chunks_mut(3).enumerate().for_each(|(idx, pixel)| {
        pixel[0] = (255.0 * 2.5 * r.data[idx] as f32 / 10000.0).clamp(0.0, 255.0) as u8;
        pixel[1] = (255.0 * 2.5 * g.data[idx] as f32 / 10000.0).clamp(0.0, 255.0) as u8;
        pixel[2] = (255.0 * 2.5 * b.data[idx] as f32 / 10000.0).clamp(0.0, 255.0) as u8;
    });

    let out = image::RgbImage::from_raw(r.width, r.height, raw).unwrap();

    for i in 0..SOURCE_DIV {
        for j in 0..SOURCE_DIV {
            crop_imm(
                &out,
                (div_size.0 * i) as u32,
                (div_size.1 * j) as u32,
                div_size.0 as u32,
                div_size.1 as u32,
            )
            .to_image()
            .save(&base_tiles_out_file_path((
                bounds.0 + ivec2(i as i32, SOURCE_DIV as i32 - 1 - j as i32) * div_bound_size,
                bounds.0
                    + ivec2(i as i32, SOURCE_DIV as i32 - 1 - j as i32) * div_bound_size
                    + div_bound_size,
            )))
            .unwrap();
        }
    }
}

fn tile_key_hash(tile_key: &TileKey) -> String {
    tile_key
        .iter()
        .map(|v| match v {
            SubDivisionKey::BottomLeft => "BL",
            SubDivisionKey::BottomRight => "BR",
            SubDivisionKey::TopLeft => "TL",
            SubDivisionKey::TopRight => "TR",
        })
        .join("_")
}

fn sub_div_tile_out_dir() -> PathBuf {
    PathBuf::from(OUT_DIR).join("sub_div")
}

fn sub_div_tile_out_file_path(tile_key: &TileKey) -> PathBuf {
    sub_div_tile_out_dir()
        .join(format!("{}_{}", tile_key.len(), tile_key_hash(tile_key)))
        .with_added_extension("jpg")
}

fn save_sub_div_image(path: &Path, image: &image::RgbImage) -> Result<(), image::ImageError> {
    let buffered_file_write = &mut BufWriter::new(File::create(path)?); // always seekable
    let encoder = JpegEncoder::new_with_quality(buffered_file_write, 95);
    encoder.write_image(
        image.as_bytes(),
        image.width(),
        image.height(),
        image::ExtendedColorType::Rgb8,
    )
}

fn merge_sub_div_tiles(tile_key: TileKey) {
    let mut merged = image::RgbImage::new(2 * SUB_DIV_RES as u32, 2 * SUB_DIV_RES as u32);

    for key in SubDivisionKey::all() {
        let src = image::open(sub_div_tile_out_file_path(&TileKey::from_iter(
            tile_key.iter().cloned().chain([key.clone()]),
        )))
        .unwrap()
        .to_rgb8();

        let offset = match key {
            SubDivisionKey::BottomLeft => usizevec2(0, SUB_DIV_RES),
            SubDivisionKey::BottomRight => usizevec2(SUB_DIV_RES, SUB_DIV_RES),
            SubDivisionKey::TopLeft => usizevec2(0, 0),
            SubDivisionKey::TopRight => usizevec2(SUB_DIV_RES, 0),
        };

        image::imageops::replace(&mut merged, &src, offset.x as i64, offset.y as i64);
    }

    let out = image::imageops::resize(
        &merged,
        SUB_DIV_RES as u32,
        SUB_DIV_RES as u32,
        image::imageops::FilterType::Lanczos3,
    );

    save_sub_div_image(sub_div_tile_out_file_path(&tile_key).as_path(), &out).unwrap();
}
fn with_progress<T, F>(label: &str, items: Vec<T>, f: F)
where
    T: Send,
    F: Fn(T) + Send + Sync,
{
    use std::io::Write;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    let total = items.len();
    let done = Arc::new(AtomicUsize::new(0));
    let done2 = Arc::clone(&done);
    let label = label.to_owned();

    let thread = std::thread::spawn(move || {
        let mut err = std::io::stderr();
        let mut last_publish = Instant::now();
        let mut last_publish_n = 0;

        loop {
            let n = done2.load(Ordering::Relaxed);
            let _ = crossterm::execute!(
                err,
                crossterm::cursor::MoveToColumn(0),
                crossterm::terminal::Clear(crossterm::terminal::ClearType::CurrentLine),
                crossterm::style::Print(format!(
                    "[{}] {}/{} (~{:.0} TPS)",
                    label,
                    n,
                    total,
                    (n - last_publish_n) as f64
                        / Instant::now().duration_since(last_publish).as_secs_f64()
                )),
            );
            last_publish_n = n;
            last_publish = Instant::now();
            if n >= total {
                let _ = writeln!(err);
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(1000));
        }
    });

    items.into_par_iter().for_each(|item| {
        f(item);
        done.fetch_add(1, Ordering::Relaxed);
    });

    thread.join().unwrap();
}

#[tokio::main]
async fn main() {
    dotenvy::dotenv().unwrap();

    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .compact()
        .init();

    let tiles: Vec<_> = std::fs::read_dir(SOURCE_DIR)
        .unwrap()
        .try_collect()
        .unwrap();
    let tiles = tiles
        .into_iter()
        .map(|t| {
            (
                t.path(),
                tile_name_to_bounds(t.file_name().to_str().unwrap()).unwrap(),
            )
        })
        .collect::<Vec<_>>();

    let base_size = tiles[0].1.1 - tiles[0].1.0;

    assert_eq!(base_size.x, base_size.y);
    assert_eq!(base_size.x % SOURCE_DIV as i32, 0);
    assert_eq!(base_size.y % SOURCE_DIV as i32, 0);

    for tile in tiles.iter() {
        assert_eq!(tile.1.1 - tile.1.0, base_size);
    }

    tokio::fs::create_dir_all(base_tiles_out_dir(base_size / SOURCE_DIV as i32))
        .await
        .unwrap();

    with_progress("base tiles", tiles, |(tile_path, bounds)| {
        process_base_tile(&tile_path, bounds);
    });

    let base_size = base_size / SOURCE_DIV as i32;

    let sub_div = SubDivision2d {
        area: DAabb2::new(dvec2(-180.0, -90.0), dvec2(180.0, 90.0)),
    };

    let base_depth = sub_div.min_depth_for_tile_count(
        base_size.as_dvec2() * (SUB_DIV_RES as f64 / (SOURCE_RES as f64 / SOURCE_DIV as f64)),
        usizevec2(1, 1),
    );

    tokio::fs::create_dir_all(sub_div_tile_out_dir())
        .await
        .unwrap();

    let base_sub_div_tiles = SubDivision2d::sub_div_keys(base_depth)
        .map(|key| (key.clone(), sub_div.tile_bbox(&key)))
        .collect_vec();

    let base_tile_image = LinearTiledImage {
        tiled_image: TiledImage {
            source: BaseTileSource {
                tiles_dir: base_tiles_out_dir(base_size),
                gcs_size: base_size,
            },
            tile_count: (ivec2(360, 180) / base_size).as_uvec2(),
            tile_resolution: UVec2::ONE * (SOURCE_RES / SOURCE_DIV) as u32,
        },
        bounds: DAabb2::new(dvec2(-180.0, -90.0), dvec2(180.0, 90.0)),
    };

    with_progress("sub-div base", base_sub_div_tiles, |(key, bounds)| {
        let out = base_tile_image.load_sub_image(bounds, UVec2::ONE * SUB_DIV_RES as u32).unwrap();
        save_sub_div_image(sub_div_tile_out_file_path(&key).as_path(), &out).unwrap();
    });

    for depth in (0..base_depth).rev() {
        let tile_keys = SubDivision2d::sub_div_keys(depth).collect_vec();

        with_progress(&format!("merge depth {depth}"), tile_keys, |key| {
            merge_sub_div_tiles(key);
        });
    }
}
