use std::fs;
use std::path::{Path, PathBuf};

use bevy::math::Vec2;
use bevy_prototype_lyon::path::ShapePath;
use geojson::{Feature, GeoJson, Geometry, GeometryValue, Position};
use walkdir::WalkDir;

pub fn load_all_shape_paths<P: AsRef<Path>>(
    root: P,
    convert_pos: impl Fn(Vec2) -> Vec2,
) -> Vec<ShapePath> {
    let paths: Vec<PathBuf> = WalkDir::new(root)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
        .filter(|entry| {
            entry
                .path()
                .extension()
                .map(|ext| ext == "geojson" || ext == "json")
                .unwrap_or(false)
        })
        .map(|entry| entry.into_path())
        .collect();

    paths
        .into_iter()
        .flat_map(|path| load_shape_paths(path, &convert_pos))
        .collect()
}

/// Iterator over ShapePaths extracted from a Natural Earth GeoJSON file
fn load_shape_paths<P: AsRef<Path>>(
    path: P,
    convert_pos: &impl Fn(Vec2) -> Vec2,
) -> impl Iterator<Item = ShapePath> {
    let data = fs::read_to_string(path).expect("Failed to read GeoJSON file");
    let geojson = data.parse::<GeoJson>().expect("Invalid GeoJSON");

    let features = match geojson {
        GeoJson::FeatureCollection(fc) => fc.features,
        _ => panic!("Expected FeatureCollection"),
    };

    features
        .into_iter()
        .flat_map(|feature| feature_to_paths(feature, convert_pos))
}

/// Convert a GeoJSON feature into one or more ShapePaths
fn feature_to_paths(feature: Feature, convert_pos: &impl Fn(Vec2) -> Vec2) -> Vec<ShapePath> {
    match feature.geometry {
        Some(geom) => geometry_to_paths(&geom, convert_pos),
        None => vec![],
    }
}

/// Convert GeoJSON geometry into ShapePaths
fn geometry_to_paths(geom: &Geometry, convert_pos: &impl Fn(Vec2) -> Vec2) -> Vec<ShapePath> {
    match &geom.value {
        GeometryValue::Polygon { coordinates } => vec![polygon_to_path(coordinates, convert_pos)],
        GeometryValue::MultiPolygon { coordinates } => coordinates
            .iter()
            .map(|x| polygon_to_path(x, convert_pos))
            .collect(),
        _ => vec![], // ignore non-polygon geometries
    }
}

/// Convert a single polygon (with rings) into a ShapePath
fn polygon_to_path(polygon: &Vec<Vec<Position>>, convert_pos: &impl Fn(Vec2) -> Vec2) -> ShapePath {
    let mut path = ShapePath::new();

    for ring in polygon {
        if ring.is_empty() {
            continue;
        }

        // Move to first point
        let first = lonlat_to_vec2(&ring[0], convert_pos);
        path = path.move_to(first);

        // Draw lines
        for coord in &ring[1..] {
            let p = lonlat_to_vec2(coord, convert_pos);
            path = path.line_to(p);
        }

        // Close ring
        path = path.close();
    }

    path
}

/// Convert [lon, lat] → Vec2
fn lonlat_to_vec2(coord: &Position, convert_pos: impl Fn(Vec2) -> Vec2) -> Vec2 {
    convert_pos(Vec2::new(coord[0] as f32, coord[1] as f32))
}
