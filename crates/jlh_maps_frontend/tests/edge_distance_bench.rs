use std::collections::HashMap;
use std::time::{Duration, Instant};

use jlh_maps_frontend::update_edge_distance_texture;
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct EdgeTileSample {
    tile_key: String,
    texture_size: usize,
    max_distance: f32,
    edge_count: usize,
    feature_count: usize,
    duration_ms: f64,
    edges: Vec<f32>,
}

#[test]
#[ignore = "local performance benchmark; run `just bench` in crates/jlh_maps_frontend"]
fn benchmark_sample_edge_tiles() {
    let samples: HashMap<String, EdgeTileSample> =
        serde_json::from_str(include_str!("test-data/sample-edge-tiles.json"))
            .expect("sample-edge-tiles.json should parse");

    assert!(
        !samples.is_empty(),
        "sample data should contain at least one tile"
    );

    let repeat_count = std::env::var("JLH_MAPS_EDGE_BENCH_REPEATS")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(3)
        .max(1);

    let mut results = samples
        .values()
        .map(|sample| benchmark_sample(sample, repeat_count))
        .collect::<Vec<_>>();

    results.sort_by(|a, b| b.best_duration.cmp(&a.best_duration));

    println!(
        "edge distance benchmark: {} tiles, {} repeats each",
        results.len(),
        repeat_count
    );
    println!(
        "{:<16} {:>8} {:>10} {:>10} {:>12} {:>12} {:>12}",
        "tile", "features", "edges", "tex", "browser_ms", "best_ms", "avg_ms"
    );

    for result in &results {
        println!(
            "{:<16} {:>8} {:>10} {:>10} {:>12.3} {:>12.3} {:>12.3}",
            result.tile_key,
            result.feature_count,
            result.edge_count,
            result.texture_size,
            result.browser_duration_ms,
            result.best_duration.as_secs_f64() * 1000.0,
            result.average_duration.as_secs_f64() * 1000.0,
        );
    }
}

struct BenchResult {
    tile_key: String,
    texture_size: usize,
    edge_count: usize,
    feature_count: usize,
    browser_duration_ms: f64,
    best_duration: Duration,
    average_duration: Duration,
}

fn benchmark_sample(sample: &EdgeTileSample, repeat_count: usize) -> BenchResult {
    assert_eq!(
        sample.edges.len(),
        sample.edge_count * 4,
        "sample edge count does not match edge buffer length for {}",
        sample.tile_key
    );

    let mut out = vec![0; sample.texture_size * sample.texture_size];
    let mut best_duration = Duration::MAX;
    let mut total_duration = Duration::ZERO;

    for _ in 0..repeat_count {
        let start = Instant::now();

        update_edge_distance_texture(
            &sample.edges,
            &mut out,
            sample.texture_size,
            sample.max_distance,
        );

        let duration = start.elapsed();
        best_duration = best_duration.min(duration);
        total_duration += duration;
    }

    BenchResult {
        tile_key: sample.tile_key.clone(),
        texture_size: sample.texture_size,
        edge_count: sample.edge_count,
        feature_count: sample.feature_count,
        browser_duration_ms: sample.duration_ms,
        best_duration,
        average_duration: total_duration / repeat_count as u32,
    }
}
