use crate::glam_ext::bounding::{AxisAlignedBoundingBox2D, DAabb2};
use anyhow::{anyhow, bail};
use geoconvert::Mgrs;
use glam::{DVec2, dvec2};
use image::{Rgba, RgbaImage};
use irox_carto::coordinate::EllipticalCoordinate;
use irox_carto::irox_units::units::length::Length;
use irox_carto::proj::Projection;
use irox_carto::tm::TransverseMercator;
use regex::Regex;
use serde::Deserialize;
use smallvec::SmallVec;
use std::simd::Simd;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct UserData {
    pub name: String,
    pub geo_footprint: GeoFootprint,
}

#[derive(Debug, Deserialize)]
pub struct GeoFootprint {
    pub coordinates: Vec<Vec<[f64; 2]>>,
}

fn sample_bilinear(img: &RgbaImage, p: DVec2) -> Rgba<u8> {
    let w = img.width() as i32;
    let h = img.height() as i32;

    let x = p.x;
    let y = p.y;

    let x0 = x.floor() as i32;
    let y0 = y.floor() as i32;
    let x1 = x0 + 1;
    let y1 = y0 + 1;

    if x0 < 0 || y0 < 0 || x1 >= w || y1 >= h {
        return Rgba([0, 0, 0, 0]);
    }

    let fx = x - x0 as f64;
    let fy = y - y0 as f64;

    let c00 = img.get_pixel(x0 as u32, y0 as u32).0;
    let c10 = img.get_pixel(x1 as u32, y0 as u32).0;
    let c01 = img.get_pixel(x0 as u32, y1 as u32).0;
    let c11 = img.get_pixel(x1 as u32, y1 as u32).0;

    let mut out = [0u8; 4];

    for i in 0..4 {
        let v = (1.0 - fx) * (1.0 - fy) * c00[i] as f64
            + fx * (1.0 - fy) * c10[i] as f64
            + (1.0 - fx) * fy * c01[i] as f64
            + fx * fy * c11[i] as f64;

        out[i] = v.clamp(0.0, 255.0) as u8;
    }

    Rgba(out)
}

/// Extract bounds from a cardinal-offset directory name, e.g. `N18E000`.
/// Each tile covers a 36° × 36° cell.
pub fn extract_bounds_offset(dir_name: &str) -> anyhow::Result<DAabb2> {
    let re = Regex::new(r"([NS])(\d+)([EW])(\d+)")?;
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

fn utm_wgs84_utm_to_transverse_mercator_params(mgrs: Mgrs) -> anyhow::Result<(DVec2, DVec2)> {
    if !mgrs.is_utm() {
        bail!("Expected UTM MGRS, got {mgrs:?}");
    }

    let central_meridian = 6. * f64::from(mgrs.zone()) - 183.;
    let latitude_of_origin = 0.0;

    let false_easting = 500_000.;
    let false_northing = if mgrs.is_north() { 0. } else { 10_000_000. };

    Ok((
        dvec2(central_meridian, latitude_of_origin),
        dvec2(false_easting, false_northing),
    ))
}

fn extract_bounds_utm(data: &UserData) -> anyhow::Result<String> {
    let r = Regex::new(r"Sentinel-2_mosaic_2025_Q3_([0-9]{2}[A-Z]{3})")?;
    if let Some(cap) = r.captures(data.name.as_str()) {
        Ok(cap[1].to_string())
    } else {
        Err(anyhow!("No UTM zone found in data name"))
    }
}

pub fn convert_sen2_img_to_epsg4326(
    src: &RgbaImage,
    data: &UserData,
) -> anyhow::Result<(RgbaImage, DAabb2)> {
    #[derive(Debug)]
    struct Bounds {
        top_left: DVec2,
        top_right: DVec2,
        bottom_right: DVec2,
        bottom_left: DVec2,
    }

    let bounds = Bounds {
        bottom_left: DVec2::from_array(data.geo_footprint.coordinates[0][0]),
        top_left: DVec2::from_array(data.geo_footprint.coordinates[0][1]),
        top_right: DVec2::from_array(data.geo_footprint.coordinates[0][2]),
        bottom_right: DVec2::from_array(data.geo_footprint.coordinates[0][3]),
    };

    let utm_100km_tile = extract_bounds_utm(data)?;
    let utm = Mgrs::parse_str(&utm_100km_tile)?;
    let (center, false_en) = utm_wgs84_utm_to_transverse_mercator_params(utm)?;

    let utm_proj = TransverseMercator::builder()
        .with_center(EllipticalCoordinate::new_degrees_wgs84(center.y, center.x))
        .with_false_easting(Length::new_meters(false_en.x))
        .with_false_northing(Length::new_meters(false_en.y))
        .build();

    let utm_top_left = utm_proj.project_to_cartesian(&EllipticalCoordinate::new_degrees_wgs84(
        bounds.top_left.y,
        bounds.top_left.x,
    ));
    let utm_top_right = utm_proj.project_to_cartesian(&EllipticalCoordinate::new_degrees_wgs84(
        bounds.top_right.y,
        bounds.top_right.x,
    ));
    let utm_bottom_right = utm_proj.project_to_cartesian(&EllipticalCoordinate::new_degrees_wgs84(
        bounds.bottom_right.y,
        bounds.bottom_right.x,
    ));
    let utm_bottom_left = utm_proj.project_to_cartesian(&EllipticalCoordinate::new_degrees_wgs84(
        bounds.bottom_left.y,
        bounds.bottom_left.x,
    ));

    let utm_aabb = DAabb2::new(
        dvec2(
            (utm_bottom_left.get_x().value() + utm_top_left.get_x().value()) / 2.0,
            (utm_bottom_left.get_y().value() + utm_bottom_right.get_y().value()) / 2.0,
        ),
        dvec2(
            (utm_bottom_right.get_x().value() + utm_top_right.get_x().value()) / 2.0,
            (utm_top_right.get_y().value() + utm_top_right.get_y().value()) / 2.0,
        ),
    );

    let rect_bounds = DAabb2::new(
        bounds
            .bottom_left
            .min(bounds.top_left)
            .min(bounds.top_right)
            .min(bounds.bottom_right),
        bounds
            .bottom_left
            .max(bounds.top_left)
            .max(bounds.top_right)
            .max(bounds.bottom_right),
    );

    let mut out = RgbaImage::new(src.width(), src.height());

    let src_size = dvec2(src.width() as f64, src.height() as f64);
    let out_size = dvec2(out.width() as f64, out.height() as f64);

    fn src_pos_batched<const N: usize>(
        xs: Simd<f64, N>,
        ys: Simd<f64, N>,
        rect_bounds: DAabb2,
        src_size: DVec2,
        out_size: DVec2,
        utm_proj: &TransverseMercator,
        utm_aabb: &DAabb2,
    ) -> (Simd<f64, N>, Simd<f64, N>) {
        let rel_x = xs / Simd::splat(out_size.x - 1.0);
        let rel_y = Simd::splat(1.0) - ys / Simd::splat(out_size.y - 1.0);

        let lon_rads = Simd::splat(rect_bounds.min().x.to_radians())
            + rel_x * Simd::splat(rect_bounds.size().x.to_radians());
        let lat_rads = Simd::splat(rect_bounds.min().y.to_radians())
            + rel_y * Simd::splat(rect_bounds.size().y.to_radians());

        let (el_x, el_y) = utm_proj.project_to_cartesian_batch(lon_rads, lat_rads);

        let src_x_rel = (el_x - Simd::splat(utm_aabb.min().x)) / Simd::splat(utm_aabb.size().x);
        let src_y_rel = Simd::splat(1.0)
            - (el_y - Simd::splat(utm_aabb.min().y)) / Simd::splat(utm_aabb.size().y);

        let src_x = src_x_rel * Simd::splat(src_size.x - 1.0);
        let src_y = src_y_rel * Simd::splat(src_size.y - 1.0);

        (sleef::f64x::rint(src_x), sleef::f64x::rint(src_y))
    }

    const N: usize = 64;

    let k = out.width() / N as u32;
    let r = out.width() % N as u32;

    for y in 0..out.height() {
        let mut update_pixel = |x: u32, y: u32, src_x: f64, src_y: f64| {
            out.put_pixel(x, y, {
                let [r, g, b, a] = sample_bilinear(
                    src,
                    dvec2(
                        src_x.clamp(0., src_size.x - 1.),
                        src_y.clamp(0., src_size.y - 1.),
                    ),
                )
                .0;
                Rgba([
                    r,
                    g,
                    b,
                    if 0.0 <= src_x
                        && src_x < src_size.x - 1.0
                        && 0.0 <= src_y
                        && src_y < src_size.y - 1.0
                    {
                        a
                    } else {
                        0
                    },
                ])
            });
        };

        for xk in 0..k {
            let xs = (0..N)
                .map(|i| (xk as usize * N + i) as f64)
                .collect::<SmallVec<[f64; N]>>();

            let (src_x, src_y) = src_pos_batched::<N>(
                Simd::from_slice(xs.as_slice()),
                Simd::splat(y as f64),
                rect_bounds,
                src_size,
                out_size,
                &utm_proj,
                &utm_aabb,
            );

            for (idx, (src_x, src_y)) in src_x
                .to_array()
                .into_iter()
                .zip(src_y.to_array())
                .enumerate()
            {
                update_pixel(xs[idx] as u32, y, src_x, src_y);
            }
        }

        for xr in 0..r {
            let x = k * N as u32 + xr;

            let (src_x, src_y) = src_pos_batched::<N>(
                Simd::splat(x as f64),
                Simd::splat(y as f64),
                rect_bounds,
                src_size,
                out_size,
                &utm_proj,
                &utm_aabb,
            );

            for (src_x, src_y) in src_x.to_array().into_iter().zip(src_y.to_array()) {
                update_pixel(x, y, src_x, src_y);
            }
        }
    }

    Ok((out, rect_bounds))
}
