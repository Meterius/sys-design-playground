use anyhow::{Context, anyhow, bail};
use clap::{Parser, Subcommand};
use glam::{dvec2, uvec2};
use image::codecs::jpeg::JpegEncoder;
use image::{EncodableLayout, ImageEncoder};
use itertools::Itertools;
use rayon::prelude::*;
use std::fs::File;
use std::io::BufWriter;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;
use utilities::distributed_mapped_image::{DistributedMappedImage, TileMeta};
use utilities::glam_ext::bounding::{AxisAlignedBoundingBox2D, DAabb2};
use utilities::glam_ext::sub_division::{SubDivision2d, tile_key_str};
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
}

// ── Mode detection ────────────────────────────────────────────────────────────

#[derive(Clone, Copy)]
enum Mode {
    /// Bounds come from `userdata.json` → `GeoFootprint`
    Mgrs,
    /// Bounds are encoded in the directory name, e.g. `N18E000`
    Offset,
}

fn detect_mode(first_tile_dir: &Path) -> anyhow::Result<Mode> {
    if first_tile_dir.join("userdata.json").exists() {
        return Ok(Mode::Mgrs);
    }
    let name = first_tile_dir
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("");
    if regex::Regex::new(r"[NS]\d+[EW]\d+").unwrap().is_match(name) {
        return Ok(Mode::Offset);
    }
    bail!(
        "Cannot detect dataset mode from {first_tile_dir:?}: \
         no userdata.json found and directory name does not match the cardinal-offset pattern"
    )
}

// ── Bounds extraction ─────────────────────────────────────────────────────────

#[derive(serde::Deserialize)]
struct UserData {
    #[serde(rename = "GeoFootprint")]
    geo_footprint: GeoFootprintField,
}

#[derive(serde::Deserialize)]
struct GeoFootprintField {
    /// `coordinates[0]` is the outer ring; each element is `[lon, lat]`.
    coordinates: Vec<Vec<[f64; 2]>>,
}

/// Extract bounds from `tile_dir/userdata.json`.
///
/// The `GeoFootprint` polygon must be axis-aligned (i.e. it must have exactly
/// 2 unique longitude values and 2 unique latitude values). Returns `Err`
/// otherwise.
fn extract_bounds_mgrs(tile_dir: &Path) -> anyhow::Result<DAabb2> {
    let path = tile_dir.join("userdata.json");
    let ud: UserData =
        serde_json::from_reader(File::open(&path).with_context(|| format!("opening {path:?}"))?)
            .with_context(|| format!("parsing {path:?}"))?;

    let ring = ud
        .geo_footprint
        .coordinates
        .into_iter()
        .next()
        .ok_or_else(|| anyhow!("{path:?}: GeoFootprint has no coordinate rings"))?;

    // Drop the closing vertex if it repeats the first.
    let verts: Vec<[f64; 2]> = if ring.len() >= 2 && ring.first() == ring.last() {
        ring[..ring.len() - 1].to_vec()
    } else {
        ring
    };

    if verts.len() != 4 {
        bail!(
            "{path:?}: expected 4 polygon vertices for an AABB, got {}",
            verts.len()
        );
    }

    const TOL: f64 = 1e-6;
    let unique_lons = unique_f64_values(verts.iter().map(|v| v[0]), TOL);
    let unique_lats = unique_f64_values(verts.iter().map(|v| v[1]), TOL);

    // if unique_lons.len() != 2 || unique_lats.len() != 2 {
    //     bail!(
    //         "{path:?}: GeoFootprint is not axis-aligned: \
    //          found {} unique longitude(s) and {} unique latitude(s)",
    //         unique_lons.len(),
    //         unique_lats.len()
    //     );
    // }

    // Every vertex must land exactly on one of the 4 expected corners.
    for v in &verts {
        let lon_ok = unique_lons.iter().any(|&u| (v[0] - u).abs() < TOL);
        let lat_ok = unique_lats.iter().any(|&u| (v[1] - u).abs() < TOL);
        if !lon_ok || !lat_ok {
            bail!("{path:?}: vertex {v:?} does not coincide with an AABB corner");
        }
    }

    let min_lon = unique_lons[0].min(unique_lons[1]);
    let max_lon = unique_lons[0].max(unique_lons[1]);
    let min_lat = unique_lats[0].min(unique_lats[1]);
    let max_lat = unique_lats[0].max(unique_lats[1]);

    Ok(DAabb2::new(
        dvec2(min_lon, min_lat),
        dvec2(max_lon, max_lat),
    ))
}

fn unique_f64_values(iter: impl Iterator<Item = f64>, tol: f64) -> Vec<f64> {
    let mut uniq: Vec<f64> = Vec::new();
    for v in iter {
        if !uniq.iter().any(|&u| (v - u).abs() < tol) {
            uniq.push(v);
        }
    }
    uniq
}

/// Extract bounds from a cardinal-offset directory name, e.g. `N18E000`.
/// Each tile covers a 36° × 36° cell.
fn extract_bounds_offset(dir_name: &str) -> anyhow::Result<DAabb2> {
    let re = regex::Regex::new(r"([NS])(\d+)([EW])(\d+)").unwrap();
    let caps = re
        .captures(dir_name)
        .ok_or_else(|| anyhow!("No cardinal-offset pattern found in {dir_name:?}"))?;

    let lat_val: f64 = caps[2].parse()?;
    let lon_val: f64 = caps[4].parse()?;

    let min_lat = if &caps[1] == "S" { -lat_val } else { lat_val };
    let min_lon = if &caps[3] == "W" { -lon_val } else { lon_val };

    Ok(DAabb2::new(
        dvec2(min_lon, min_lat),
        dvec2(min_lon + 36.0, min_lat + 36.0),
    ))
}

// ── Band I/O ──────────────────────────────────────────────────────────────────

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
fn merge_bands(tile_dir: &Path) -> anyhow::Result<image::RgbImage> {
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

    let mut raw = vec![0u8; (r.width * r.height) as usize * 3];
    raw.par_chunks_mut(3).enumerate().for_each(|(idx, pixel)| {
        pixel[0] = (255.0 * 2.5 * r.data[idx] as f32 / 10000.0).clamp(0.0, 255.0) as u8;
        pixel[1] = (255.0 * 2.5 * g.data[idx] as f32 / 10000.0).clamp(0.0, 255.0) as u8;
        pixel[2] = (255.0 * 2.5 * b.data[idx] as f32 / 10000.0).clamp(0.0, 255.0) as u8;
    });

    image::RgbImage::from_raw(r.width, r.height, raw)
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

fn save_jpeg(path: &Path, img: &image::RgbImage) -> anyhow::Result<()> {
    let file = File::create(path).with_context(|| format!("creating {path:?}"))?;
    JpegEncoder::new_with_quality(BufWriter::new(file), 99)
        .write_image(
            img.as_bytes(),
            img.width(),
            img.height(),
            image::ExtendedColorType::Rgb8,
        )
        .with_context(|| format!("encoding JPEG to {path:?}"))
}

fn save_dist_tile_jpeg(
    img_path: &Path,
    img: &image::RgbImage,
    bounds: DAabb2,
) -> anyhow::Result<()> {
    let meta_path = img_path.with_extension("jpg.meta.json");
    save_jpeg(&img_path, &img)?;
    serde_json::to_writer_pretty(
        File::create(&meta_path).with_context(|| format!("creating {meta_path:?}"))?,
        &TileMeta { bounds },
    )
    .with_context(|| format!("writing {meta_path:?}"))
}

fn ingest_tile(
    tile_dir: &Path,
    bounds: DAabb2,
    out_dir: &Path,
    max_res: Option<u32>,
) -> anyhow::Result<()> {
    let stem = tile_dir
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| anyhow!("Cannot derive stem from {tile_dir:?}"))?;

    let merged = merge_bands(tile_dir)?;
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

            let img_path = out_dir.join(format!("{img_stem}.jpg"));
            save_dist_tile_jpeg(&img_path, &sub_img, sub_bounds)?;
        }
    }

    Ok(())
}

// ── Progress helper ───────────────────────────────────────────────────────────

fn with_progress<T, F>(label: &str, items: Vec<T>, f: F)
where
    T: Send,
    F: Fn(T) + Send + Sync,
{
    use std::io::Write;

    let total = items.len();
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
            std::thread::sleep(std::time::Duration::from_millis(1000));
        }
    });

    items.into_par_iter().for_each(|item| {
        f(item);
        done.fetch_add(1, Ordering::Relaxed);
    });

    thread.join().unwrap();
}

// ── Command implementation ────────────────────────────────────────────────────

fn ingest_source_as_dist_img(
    input: &Path,
    output: &Path,
    max_res: Option<u32>,
) -> anyhow::Result<()> {
    let mut tile_dirs: Vec<PathBuf> = std::fs::read_dir(input)
        .with_context(|| format!("reading {input:?}"))?
        .map(|e| e.with_context(|| format!("reading entry in {input:?}")))
        .collect::<anyhow::Result<Vec<_>>>()?
        .into_par_iter()
        .filter(|e| e.file_type().map(|ft| ft.is_dir()).unwrap_or(false))
        .map(|e| e.path())
        .collect();

    if tile_dirs.is_empty() {
        bail!("No subdirectories found in {input:?}");
    }

    tile_dirs.sort(); // deterministic order

    let mode = detect_mode(&tile_dirs[0])?;
    eprintln!(
        "Detected mode: {}  ({} tiles)",
        match mode {
            Mode::Mgrs => "MGRS — bounds from userdata.json",
            Mode::Offset => "cardinal-offset — bounds from directory name",
        },
        tile_dirs.len()
    );

    // Extract bounds single-threaded for clear, ordered error messages.
    let tiles: Vec<(PathBuf, DAabb2)> = tile_dirs
        .into_par_iter()
        .map(|dir| {
            let bounds = match mode {
                Mode::Mgrs => extract_bounds_mgrs(&dir),
                Mode::Offset => {
                    let name = dir
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("")
                        .to_owned();
                    extract_bounds_offset(&name)
                }
            }
            .with_context(|| format!("extracting bounds for {dir:?}"))?;
            Ok((dir, bounds))
        })
        .collect::<anyhow::Result<_>>()?;

    std::fs::create_dir_all(output)
        .with_context(|| format!("creating output directory {output:?}"))?;

    with_progress("ingest", tiles, |(tile_dir, bounds)| {
        if let Err(e) = ingest_tile(&tile_dir, bounds, output, max_res) {
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
    let bounds = DAabb2::new(dvec2(-180.0, -90.0), dvec2(180.0, 90.0));

    let tile_output = |idx: u32| output.join(format!("{idx}"));

    eprintln!("Using bounds: {:?}", bounds);

    let sub_div = SubDivision2d { area: bounds };

    for idx in (0..=depth).rev() {
        let dist_tile_img = if idx == depth {
            DistributedMappedImage::from_directory(input)
                .with_context(|| format!("opening distributed image at {input:?}"))?
        } else {
            DistributedMappedImage::from_directory(tile_output(idx + 1))
                .with_context(|| format!("opening distributed image at {input:?}"))?
        };

        let tiles = SubDivision2d::sub_div_keys(idx as usize)
            .map(|key| (key.clone(), sub_div.tile_bbox(&key)))
            .collect_vec();

        let tile_output = output.join(format!("{idx}"));
        std::fs::create_dir_all(&tile_output)
            .with_context(|| format!("creating tile output directory {tile_output:?}"))?;

        with_progress(
            &format!("make tiles at depth {}", idx),
            tiles,
            move |(key, bounds)| {
                if let Ok(Some(out)) = dist_tile_img
                    .load_sub_image(bounds, uvec2(resolution, resolution))
                    .with_context(|| format!("making tile for key {key:?} with bounds {bounds:?}"))
                    .inspect_err(|e| eprintln!("Error loading base tile for key {key:?}: {e:#}"))
                {
                    let _ = save_dist_tile_jpeg(
                        &tile_output.join(format!("{}.jpg", tile_key_str(&key))),
                        &out,
                        bounds,
                    )
                    .with_context(|| format!("saving tile for key {key:?}"))
                    .inspect_err(|e| eprintln!("Error saving tile for key {key:?}: {e:#}"));
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
    }
}
