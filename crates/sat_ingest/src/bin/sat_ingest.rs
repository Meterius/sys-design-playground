use anyhow::{Context, anyhow, bail};
use clap::{Parser, Subcommand, ValueEnum};
use glam::{DVec2, USizeVec2, dvec2, uvec2};
use image::codecs::png::{CompressionType, FilterType, PngEncoder};
use image::{EncodableLayout, ImageEncoder};
use itertools::Itertools;
use rayon::prelude::*;
use sat_ingest::sen2::{
    UserData, convert_sen2_img_to_epsg4326, load_dist_img_web_mercator_tile, web_mercator_max_lat,
};
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::ErrorKind;
use std::io::{BufWriter, Cursor};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;
use utilities::distributed_mapped_image::{DistributedMappedImage, TileMeta};
use utilities::glam_ext::bounding::{AxisAlignedBoundingBox2D, DAabb2};
use utilities::glam_ext::sub_division::{SubDivision2d, tile_key_str};
use utilities::queued_reading::QueuedReader;

// ── CLI ──────────────────────────────────────────────────────────────────────

#[derive(Parser)]
#[command(name = "sat_ingest")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Ingest a directory of raw satellite geo-tiles (MGRS or cardinal-offset)
    /// into a flat directory of merged-band JPEG images with companion
    /// `.meta.json` files suitable for `DistributedMappedImage::from_directory`.
    IngestSourceAsDistImg {
        /// Root directory containing per-tile subdirectories (each with B02/B03/B04.tif)
        #[arg(long)]
        input: PathBuf,
        /// Output directory for merged images and meta files
        #[arg(long)]
        output: PathBuf,
        /// Maximum pixel dimension for any output tile. If the merged image
        /// exceeds this in width or height it will be split into sub-tiles
        /// whose dimensions evenly divide the source resolution.
        #[arg(long)]
        max_resolution: Option<u32>,
    },
    /// Ingest a `DistributedMappedImage::from_directory` and process them hierarchically subdivided equirectangular tiles.
    ProcessDistImgToHierarchicalTiles {
        /// Root directory containing a distributed image
        #[arg(long)]
        input: PathBuf,
        /// Output directory for hierarchical tiles and meta files
        #[arg(long)]
        output: PathBuf,
        /// Depth of the smallest output tiles
        #[arg(long)]
        depth: u32,
        /// Resolution of each output tile
        #[arg(long)]
        resolution: u32,
    },
    /// Ingest a `DistributedMappedImage::from_directory` and process it into TileJSON-compatible Web Mercator XYZ tiles.
    #[command(name = "process_dist_img_into_tile_json")]
    ProcessDistImgIntoTileJson {
        /// Root directory containing a distributed image
        #[arg(long)]
        input: PathBuf,
        /// Output directory for TileJSON and XYZ tile files
        #[arg(long)]
        output: PathBuf,
        /// Maximum output zoom level
        #[arg(long)]
        depth: u32,
        /// Resume processing from a given depth
        #[arg(long)]
        resume_depth: Option<u32>,
        /// Resolution of each output tile
        #[arg(long)]
        resolution: u32,
        /// Resize filter used when stitching lower zoom tiles from child tiles
        #[arg(long, value_enum, default_value_t = StitchFilter::Triangle)]
        stitch_filter: StitchFilter,
        #[arg(long, default_value = "/")]
        tile_dir_url: String,

    },
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum StitchFilter {
    Nearest,
    Triangle,
    CatmullRom,
    Gaussian,
    Lanczos3,
}

impl StitchFilter {
    fn to_image_filter(self) -> image::imageops::FilterType {
        match self {
            StitchFilter::Nearest => image::imageops::FilterType::Nearest,
            StitchFilter::Triangle => image::imageops::FilterType::Triangle,
            StitchFilter::CatmullRom => image::imageops::FilterType::CatmullRom,
            StitchFilter::Gaussian => image::imageops::FilterType::Gaussian,
            StitchFilter::Lanczos3 => image::imageops::FilterType::Lanczos3,
        }
    }
}

// ── Mode detection ────────────────────────────────────────────────────────────

#[derive(Clone, Copy)]
enum Mode {
    /// Bounds come from `userdata.json` → `GeoFootprint`
    Mgrs,
    /// Bounds are encoded in the directory name, e.g. `N18E000`
    Offset,
}

impl Mode {
    fn dir_files(&self) -> HashSet<String> {
        match self {
            Mode::Mgrs => HashSet::from_iter(
                ["B02.tif", "B03.tif", "B04.tif", "userdata.json"]
                    .into_iter()
                    .map(|s| s.to_owned()),
            ),
            Mode::Offset => HashSet::from_iter(
                ["B02.tif", "B03.tif", "B04.tif"]
                    .into_iter()
                    .map(|s| s.to_owned()),
            ),
        }
    }
}

fn detect_mode(first_tile_dir: &Path) -> anyhow::Result<Mode> {
    if first_tile_dir.join("userdata.json").exists() {
        return Ok(Mode::Mgrs);
    }
    let name = first_tile_dir
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("");
    if regex::Regex::new(r"[NS]\d+[EW]\d+")?.is_match(name) {
        return Ok(Mode::Offset);
    }
    bail!(
        "Cannot detect dataset mode from {first_tile_dir:?}: \
         no userdata.json found and directory name does not match the cardinal-offset pattern"
    )
}

// ── Band I/O ──────────────────────────────────────────────────────────────────

struct TiffBand {
    width: u32,
    height: u32,
    data: Vec<i16>,
}

fn read_tiff_band(path: PathBuf, data: Vec<u8>) -> anyhow::Result<TiffBand> {
    let mut decoder = tiff::decoder::Decoder::new(Cursor::new(data))
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
fn merge_bands(
    r: Vec<u8>,
    g: Vec<u8>,
    b: Vec<u8>,
    tile_dir: &Path,
) -> anyhow::Result<image::RgbaImage> {
    let ((r, g), b) = rayon::join(
        || {
            rayon::join(
                || read_tiff_band(tile_dir.join("B04.tif"), r),
                || read_tiff_band(tile_dir.join("B03.tif"), g),
            )
        },
        || read_tiff_band(tile_dir.join("B02.tif"), b),
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

// ── Subdivision ───────────────────────────────────────────────────────────────

/// Returns the smallest divisor of `n` that is ≥ ⌈n / max⌉, ensuring that
/// `n / d ≤ max`. `n` is always a valid answer (d=n → 1 pixel tile), so this
/// never fails in practice.
fn smallest_sufficient_divisor(n: u32, max: u32) -> u32 {
    let min_d = n.div_ceil(max);
    (min_d..=n).find(|&d| n.is_multiple_of(d)).unwrap_or(n)
}

// ── Output ────────────────────────────────────────────────────────────────────

fn save_png(path: &Path, img: &image::RgbaImage) -> anyhow::Result<()> {
    let file = File::create(path).with_context(|| format!("creating {path:?}"))?;
    PngEncoder::new_with_quality(
        BufWriter::new(file),
        CompressionType::Fast,
        FilterType::Adaptive,
    )
    .write_image(
        img.as_bytes(),
        img.width(),
        img.height(),
        image::ExtendedColorType::Rgba8,
    )
    .with_context(|| format!("encoding PNG to {path:?}"))
}

fn save_dist_tile_png(
    img_path: &Path,
    img: &image::RgbaImage,
    bounds: DAabb2,
) -> anyhow::Result<()> {
    let meta_path = img_path.with_extension("png.meta.json");
    save_png(img_path, img)?;
    serde_json::to_writer_pretty(
        File::create(&meta_path).with_context(|| format!("creating {meta_path:?}"))?,
        &TileMeta { bounds },
    )
    .with_context(|| format!("writing {meta_path:?}"))
}

fn ingest_tile(
    tile_dir: &Path,
    mut tile_dir_files: HashMap<String, Vec<u8>>,
    mode: Mode,
    out_dir: &Path,
    max_res: Option<u32>,
) -> anyhow::Result<()> {
    let stem = tile_dir
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| anyhow!("Cannot derive stem from {tile_dir:?}"))?;

    let interest_bounds = DAabb2::new(
        dvec2(-180., -90.),
        dvec2(180., 90.),
        // dvec2(5.3236, 49.5195),
        // dvec2(15.686, 55.148)
    );

    let (merged, bounds) = match mode {
        Mode::Mgrs => {
            let ud: UserData = serde_json::from_reader(
                tile_dir_files
                    .remove("userdata.json")
                    .ok_or(anyhow!("Missing userdata.json"))?
                    .as_slice(),
            )
            .with_context(|| "parsing userdata.json".to_string())?;

            let bounds = DAabb2::new(
                ud.geo_footprint.coordinates[0]
                    .iter()
                    .map(|v| DVec2::from_array(*v))
                    .reduce(DVec2::min)
                    .unwrap(),
                ud.geo_footprint.coordinates[0]
                    .iter()
                    .map(|v| DVec2::from_array(*v))
                    .reduce(DVec2::max)
                    .unwrap(),
            );

            if bounds.intersection(interest_bounds).is_none() {
                return Ok(());
            }

            let r = tile_dir_files
                .remove("B04.tif")
                .ok_or(anyhow!("Missing B04.tif"))?;
            let g = tile_dir_files
                .remove("B03.tif")
                .ok_or(anyhow!("Missing B03.tif"))?;
            let b = tile_dir_files
                .remove("B02.tif")
                .ok_or(anyhow!("Missing B02.tif"))?;

            let merged = merge_bands(r, g, b, tile_dir)?;
            convert_sen2_img_to_epsg4326(&merged, &ud)?
        }
        Mode::Offset => {
            bail!("fck this")
            // let name = tile_dir
            //     .file_name()
            //     .and_then(|n| n.to_str())
            //     .unwrap_or("")
            //     .to_owned();
            // let merged = merge_bands(tile_dir)?;
            // (merged, extract_bounds_offset(&name)?)
        }
    };

    let (w, h) = (merged.width(), merged.height());

    let (d_w, d_h) = match max_res {
        Some(r) => (
            smallest_sufficient_divisor(w, r),
            smallest_sufficient_divisor(h, r),
        ),
        None => (1, 1),
    };

    let tile_w = w / d_w;
    let tile_h = h / d_h;
    let lon_step = bounds.size().x / d_w as f64;
    let lat_step = bounds.size().y / d_h as f64;

    for j in 0..d_h {
        for i in 0..d_w {
            let (img_stem, sub_bounds) = if d_w == 1 && d_h == 1 {
                (stem.to_owned(), bounds)
            } else {
                // j=0 → southernmost (world Y-min) → bottom pixel rows
                let sub_min = bounds.min() + dvec2(i as f64 * lon_step, j as f64 * lat_step);
                (
                    format!("{stem}_{i}_{j}"),
                    DAabb2::new(sub_min, sub_min + dvec2(lon_step, lat_step)),
                )
            };

            // Pixel row 0 = world Y-max; j=0 = world Y-min → crop from the bottom.
            let crop_x = i * tile_w;
            let crop_y = (d_h - 1 - j) * tile_h;
            let sub_img =
                image::imageops::crop_imm(&merged, crop_x, crop_y, tile_w, tile_h).to_image();

            let img_path = out_dir.join(format!("{img_stem}.png"));
            save_dist_tile_png(&img_path, &sub_img, sub_bounds)?;
        }
    }

    Ok(())
}

// ── Progress helper ───────────────────────────────────────────────────────────

fn with_progress<T, F>(label: &str, total: usize, items: impl ParallelIterator<Item = T>, f: F)
where
    T: Send,
    F: Fn(T) + Send + Sync,
{
    use std::io::Write;

    let done = Arc::new(AtomicUsize::new(0));
    let done2 = Arc::clone(&done);
    let label = label.to_owned();

    let thread = std::thread::spawn(move || {
        let mut err = std::io::stderr();
        let mut last_n = 0usize;
        let mut last_t = Instant::now();
        loop {
            let n = done2.load(Ordering::Relaxed);
            let now = Instant::now();
            let tps = (n - last_n) as f64 / now.duration_since(last_t).as_secs_f64();
            let _ = crossterm::execute!(
                err,
                crossterm::cursor::MoveToColumn(0),
                crossterm::terminal::Clear(crossterm::terminal::ClearType::CurrentLine),
                crossterm::style::Print(format!("[{label}] {n}/{total} (~{tps:.4} TPS)"))
            );
            last_n = n;
            last_t = now;
            if n >= total {
                let _ = writeln!(err);
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(10000));
        }
    });

    items.for_each(|item| {
        f(item);
        done.fetch_add(1, Ordering::Relaxed);
    });

    done.store(total, Ordering::Relaxed);

    thread.join().unwrap();
}

// ── Command implementation ────────────────────────────────────────────────────

fn ingest_source_as_dist_img(
    input: &Path,
    output: &Path,
    max_res: Option<u32>,
) -> anyhow::Result<()> {
    fn tile_dir_completed_path(output: &Path, tile_dir: &Path) -> PathBuf {
        output.join(format!(
            "{}_completed",
            tile_dir.file_name().unwrap().to_str().unwrap()
        ))
    }

    let tile_dirs: Vec<PathBuf> = std::fs::read_dir(input)
        .with_context(|| format!("reading {input:?}"))?
        .map(|e| e.with_context(|| format!("reading entry in {input:?}")))
        .collect::<anyhow::Result<Vec<_>>>()?
        .into_iter()
        .filter(|e| e.file_type().map(|ft| ft.is_dir()).unwrap_or(false))
        .map(|e| e.path())
        .collect();

    let total = tile_dirs.len();

    if tile_dirs.is_empty() {
        bail!("No subdirectories found in {input:?}");
    }

    let mode = detect_mode(&tile_dirs[0])?;
    let tile_dir_files = mode.dir_files();

    eprintln!(
        "Detected mode: {}  ({} tiles)",
        match mode {
            Mode::Mgrs => "MGRS — bounds from userdata.json",
            Mode::Offset => "cardinal-offset — bounds from directory name",
        },
        tile_dirs.len()
    );

    std::fs::create_dir_all(output)
        .with_context(|| format!("creating output directory {output:?}"))?;

    let (tx, rx) = crossbeam::channel::bounded(8);

    {
        let output = output.to_owned();
        rayon::spawn(move || {
            let read_dir = |dir: &PathBuf| -> anyhow::Result<HashMap<String, Vec<u8>>> {
                let mut files = HashMap::new();

                for entry in std::fs::read_dir(dir).context("reading directory")? {
                    let entry = entry.context("reading directory entry")?;
                    if entry.file_type().context("getting file type")?.is_file()
                        && tile_dir_files.contains(entry.file_name().to_str().unwrap())
                    {
                        let path = entry.path();
                        let file_bytes = std::fs::read(&path).context("reading file")?;
                        files.insert(
                            path.file_name().unwrap().to_str().unwrap().to_owned(),
                            file_bytes,
                        );
                    }
                }

                Ok(files)
            };

            for dir in tile_dirs {
                if !std::fs::exists(tile_dir_completed_path(&output, &dir)).unwrap() {
                    match read_dir(&dir) {
                        Ok(files) => {
                            tx.send(Some((dir.clone(), files))).unwrap();
                        }
                        Err(err) => {
                            tx.send(None).unwrap();
                            eprintln!("Error reading directory {dir:?}: {err:#}");
                        }
                    }
                } else {
                    tx.send(None).unwrap();
                }
            }
        });
    }

    with_progress("ingest", total, rx.into_iter().par_bridge(), |item| {
        if let Some((tile_dir, files)) = item
            && let Err(e) = ingest_tile(&tile_dir, files, mode, output, max_res)
                .map(|_| std::fs::write(tile_dir_completed_path(output, &tile_dir), b""))
        {
            eprintln!("\nError ingesting {tile_dir:?}: {e:#}");
        }
    });

    Ok(())
}

fn process_dist_img_into_hierarchical_tiles(
    input: &Path,
    output: &Path,
    resolution: u32,
    depth: u32,
) -> anyhow::Result<()> {
    let source_resolution = uvec2(1668, 1668);
    let resolution = uvec2(resolution, resolution);

    let bounds = DAabb2::new(dvec2(-180.0, -90.0), dvec2(180.0, 90.0));

    let tile_output = |idx: u32| output.join(format!("{idx}"));

    eprintln!("Using bounds: {:?}", bounds);

    let sub_div = SubDivision2d { area: bounds };

    let target_depth = sub_div.min_depth_for_tile_count(
        resolution.as_dvec2() / source_resolution.as_dvec2(),
        USizeVec2::ONE,
    );

    eprintln!(
        "Assuming source resolution {source_resolution:?} target depth for {resolution:?} is {target_depth}"
    );

    for idx in (0..=depth).rev() {
        let dist_tile_img = if idx == depth {
            DistributedMappedImage::from_directory(input, false)
                .with_context(|| format!("opening distributed image at {input:?}"))?
        } else {
            DistributedMappedImage::from_directory(tile_output(idx + 1), false)
                .with_context(|| format!("opening distributed image at {input:?}"))?
        };

        if dist_tile_img.bounds().is_none() {
            eprintln!("Skipping depth {idx}: no source images available");
            continue;
        }

        let tiles = SubDivision2d::sub_div_keys(idx as usize)
            .map(|key| (key.clone(), sub_div.tile_bbox(&key)))
            .collect_vec();

        let tile_output = output.join(format!("{idx}"));
        std::fs::create_dir_all(&tile_output)
            .with_context(|| format!("creating tile output directory {tile_output:?}"))?;

        with_progress(
            &format!("make tiles at depth {}", idx),
            tiles.len(),
            tiles.into_par_iter(),
            move |(key, bounds)| {
                let key = tile_key_str(&key);
                let key = if key.is_empty() { "R".into() } else { key };

                let tile_output_tile_path = tile_output.join(format!("{}.png", key));

                if !std::fs::exists(&tile_output_tile_path).unwrap()
                    && let Ok(Some(out)) = dist_tile_img
                        .load_sub_image(bounds, resolution)
                        .with_context(|| {
                            format!("making tile for key {key:?} with bounds {bounds:?}")
                        })
                        .inspect_err(|e| {
                            eprintln!("Error loading base tile for key {key:?}: {e:#}")
                        })
                {
                    let _ = save_dist_tile_png(&tile_output_tile_path, &out, bounds)
                        .with_context(|| format!("saving tile for key {key:?}"))
                        .inspect_err(|e| eprintln!("Error saving tile for key {key:?}: {e:#}"));
                }
            },
        );
    }

    Ok(())
}

#[derive(Serialize)]
struct TileJson {
    tilejson: &'static str,
    name: &'static str,
    tiles: Vec<String>,
    bounds: [f64; 4],
    scheme: &'static str,
    minzoom: u32,
    maxzoom: u32,
    #[serde(rename = "tileSize")]
    tile_size: u32,
}

fn xyz_tile_range_for_bounds(bounds_degrees: DAabb2, z: u32) -> (u32, u32, u32, u32) {
    let max_lat = web_mercator_max_lat().to_degrees();
    let gcs_bounds_radians = DAabb2::new(
        dvec2(
            bounds_degrees.min().x.clamp(-180.0, 180.0).to_radians(),
            bounds_degrees.min().y.clamp(-max_lat, max_lat).to_radians(),
        ),
        dvec2(
            bounds_degrees.max().x.clamp(-180.0, 180.0).to_radians(),
            bounds_degrees.max().y.clamp(-max_lat, max_lat).to_radians(),
        ),
    );
    let mercator_y = |lat: f64| (std::f64::consts::FRAC_PI_4 + lat / 2.0).tan().ln();
    let mercator_bounds = DAabb2::new(
        dvec2(
            gcs_bounds_radians.min().x,
            mercator_y(gcs_bounds_radians.min().y),
        ),
        dvec2(
            gcs_bounds_radians.max().x,
            mercator_y(gcs_bounds_radians.max().y),
        ),
    );

    let n = 2_u32.pow(z);
    let n_f = n as f64;
    let tile_size = 2.0 * std::f64::consts::PI / n_f;

    let x_min = ((mercator_bounds.min().x + std::f64::consts::PI) / tile_size)
        .floor()
        .clamp(0.0, n_f - 1.0) as u32;
    let x_max = (((mercator_bounds.max().x + std::f64::consts::PI) / tile_size).ceil() - 1.0)
        .clamp(0.0, n_f - 1.0) as u32;
    let y_min = ((std::f64::consts::PI - mercator_bounds.max().y) / tile_size)
        .floor()
        .clamp(0.0, n_f - 1.0) as u32;
    let y_max = (((std::f64::consts::PI - mercator_bounds.min().y) / tile_size).ceil() - 1.0)
        .clamp(0.0, n_f - 1.0) as u32;

    (x_min, x_max, y_min, y_max)
}

fn xyz_tile_path(output: &Path, z: u32, x: u32, y: u32) -> PathBuf {
    output.join(format!("{z}/{x}/{y}.png"))
}

fn stitch_xyz_parent_tile(
    queued_reader: &QueuedReader,
    output: &Path,
    z: u32,
    x: u32,
    y: u32,
    resolution: u32,
    filter: image::imageops::FilterType,
) -> anyhow::Result<Option<image::RgbaImage>> {
    let mut out = image::RgbaImage::new(resolution, resolution);
    let child_resolution = resolution / 2;
    let mut has_child = false;

    for child_y_offset in 0..2 {
        for child_x_offset in 0..2 {
            let child_path = xyz_tile_path(
                output,
                z + 1,
                x * 2 + child_x_offset,
                y * 2 + child_y_offset,
            );

            let child_bytes = match queued_reader.blocking_read(child_path.clone()) {
                Ok(bytes) => bytes,
                Err(err) if err.kind() == ErrorKind::NotFound => continue,
                Err(err) => {
                    return Err(err).with_context(|| format!("reading child tile {child_path:?}"));
                }
            };

            has_child = true;
            let child = image::load_from_memory(&child_bytes)
                .with_context(|| format!("decoding child tile {child_path:?}"))?
                .to_rgba8();
            let child = image::imageops::resize(&child, child_resolution, child_resolution, filter);

            image::imageops::overlay(
                &mut out,
                &child,
                (child_x_offset * child_resolution) as i64,
                (child_y_offset * child_resolution) as i64,
            );
        }
    }

    Ok(has_child.then_some(out))
}

fn process_dist_img_into_tile_json(
    input: &Path,
    output: &Path,
    resolution: u32,
    depth: u32,
    stitch_filter: StitchFilter,
    resume_depth: Option<u32>,
    tile_dir_url: String,
) -> anyhow::Result<()> {
    anyhow::ensure!(
        depth <= 30,
        "depth {depth} is too high for u32 XYZ tile indexing"
    );
    anyhow::ensure!(
        resolution.is_multiple_of(2),
        "resolution {resolution} must be even so parent tiles can be stitched from four children"
    );

    if let Some(resume_depth) = resume_depth {
        eprintln!("Resuming at depth {}...", resume_depth);
    }

    let resume_depth = resume_depth.unwrap_or(depth);

    std::fs::create_dir_all(output)
        .with_context(|| format!("creating output directory {output:?}"))?;

    let dist_img = DistributedMappedImage::from_directory(input, true)
        .with_context(|| format!("opening distributed image at {input:?}"))?;
    let source_bounds = dist_img
        .bounds()
        .ok_or_else(|| anyhow!("No source tiles found in distributed image at {input:?}"))?;
    let max_lat = web_mercator_max_lat().to_degrees();
    let tilejson = TileJson {
        tilejson: "3.0.0",
        name: "satellite",
        tiles: vec![format!("{tile_dir_url}/{{z}}/{{x}}/{{y}}.png")],
        bounds: [
            source_bounds.min().x.clamp(-180.0, 180.0),
            source_bounds.min().y.clamp(-max_lat, max_lat),
            source_bounds.max().x.clamp(-180.0, 180.0),
            source_bounds.max().y.clamp(-max_lat, max_lat),
        ],
        scheme: "xyz",
        minzoom: 0,
        maxzoom: depth,
        tile_size: resolution,
    };

    let tilejson_path = output.join("tilejson.json");
    serde_json::to_writer_pretty(
        File::create(&tilejson_path).with_context(|| format!("creating {tilejson_path:?}"))?,
        &tilejson,
    )
    .with_context(|| format!("writing {tilejson_path:?}"))?;

    // Base Tiles

    if resume_depth >= depth {
        let (x_min, x_max, y_min, y_max) = xyz_tile_range_for_bounds(source_bounds, depth);
        let base_tiles = (x_min..=x_max)
            .flat_map(|x| (y_min..=y_max).map(move |y| (x, y)))
            .collect_vec();

        with_progress(
            &format!("make tilejson xyz base tiles at zoom {depth}"),
            base_tiles.len(),
            base_tiles.into_par_iter(),
            |(x, y)| {
                let tile_path = xyz_tile_path(output, depth, x, y);
                if std::fs::exists(&tile_path).unwrap_or(false) {
                    return;
                }

                if let Some(parent) = tile_path.parent()
                    && let Err(err) = std::fs::create_dir_all(parent)
                {
                    eprintln!("Error creating tile directory {parent:?}: {err:#}");
                    return;
                }

                let tile = match load_dist_img_web_mercator_tile(&dist_img, depth, x, y, resolution)
                    .with_context(|| format!("making tile z={depth} x={x} y={y}"))
                    .inspect_err(|err| eprintln!("Error making tile z={depth} x={x} y={y}: {err:#}"))
                {
                    Ok(Some(tile)) => tile,
                    Ok(None) => return,
                    Err(_) => return,
                };

                if let Err(err) = save_png(&tile_path, &tile)
                    .with_context(|| format!("saving tile z={depth} x={x} y={y} to {tile_path:?}"))
                {
                    eprintln!("Error saving tile z={depth} x={x} y={y}: {err:#}");
                }
            },
        );
    }

    // Merge Tiles

    let tile_reader = Arc::new(QueuedReader::new());

    for z in (0..=resume_depth.min(depth.saturating_sub(1))).rev() {
        let (x_min, x_max, y_min, y_max) = xyz_tile_range_for_bounds(source_bounds, z);
        let parent_tiles = (x_min..=x_max)
            .flat_map(|x| (y_min..=y_max).map(move |y| (x, y)))
            .collect_vec();
        let tile_reader = Arc::clone(&tile_reader);

        with_progress(
            &format!("stitch tilejson xyz tiles at zoom {z}"),
            parent_tiles.len(),
            parent_tiles.into_par_iter(),
            move |(x, y)| {
                let tile_path = xyz_tile_path(output, z, x, y);
                if std::fs::exists(&tile_path).unwrap_or(false) {
                    return;
                }

                if let Some(parent) = tile_path.parent()
                    && let Err(err) = std::fs::create_dir_all(parent)
                {
                    eprintln!("Error creating tile directory {parent:?}: {err:#}");
                    return;
                }

                let tile = match stitch_xyz_parent_tile(
                    tile_reader.as_ref(),
                    output,
                    z,
                    x,
                    y,
                    resolution,
                    stitch_filter.to_image_filter(),
                )
                .with_context(|| format!("stitching tile z={z} x={x} y={y}"))
                {
                    Ok(Some(tile)) => tile,
                    Ok(None) => return,
                    Err(err) => {
                        eprintln!("Error stitching tile z={z} x={x} y={y}: {err:#}");
                        return;
                    }
                };

                if let Err(err) = save_png(&tile_path, &tile)
                    .with_context(|| format!("saving tile z={z} x={x} y={y} to {tile_path:?}"))
                {
                    eprintln!("Error saving tile z={z} x={x} y={y}: {err:#}");
                }
            },
        );
    }

    Ok(())
}

// ── Entry point ───────────────────────────────────────────────────────────────

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::IngestSourceAsDistImg {
            input,
            output,
            max_resolution,
        } => ingest_source_as_dist_img(&input, &output, max_resolution),
        Commands::ProcessDistImgToHierarchicalTiles {
            input,
            output,
            resolution,
            depth,
        } => process_dist_img_into_hierarchical_tiles(&input, &output, resolution, depth),
        Commands::ProcessDistImgIntoTileJson {
            input,
            output,
            resolution,
            depth,
            stitch_filter,
            resume_depth,
            tile_dir_url,
        } => process_dist_img_into_tile_json(&input, &output, resolution, depth, stitch_filter, resume_depth, tile_dir_url),
    }
}
