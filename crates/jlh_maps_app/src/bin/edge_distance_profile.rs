use std::collections::HashMap;
use std::hint::black_box;
use std::time::{Duration, Instant};

use jlh_maps_app::utils::edge_distance::update_edge_distance_texture;
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

fn main() {
    let samples: HashMap<String, EdgeTileSample> = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/test-data/sample-edge-tiles.json"
    )))
    .expect("sample-edge-tiles.json should parse");

    let repeat_count = env_usize("JLH_MAPS_EDGE_PROFILE_REPEATS")
        .unwrap_or(10)
        .max(1);
    let tile_filter = std::env::var("JLH_MAPS_EDGE_PROFILE_TILE").ok();
    let mut selected_samples = samples
        .values()
        .filter(|sample| {
            tile_filter
                .as_ref()
                .is_none_or(|tile_key| tile_key == &sample.tile_key)
        })
        .collect::<Vec<_>>();

    selected_samples.sort_by(|a, b| b.duration_ms.total_cmp(&a.duration_ms));

    assert!(
        !selected_samples.is_empty(),
        "no samples selected; check JLH_MAPS_EDGE_PROFILE_TILE"
    );

    println!(
        "profiling {} tile(s), {} repeat(s) each",
        selected_samples.len(),
        repeat_count
    );

    let profile_start = Instant::now();
    let mut checksum = 0u64;
    let mut total_duration = Duration::ZERO;

    for repeat_index in 0..repeat_count {
        for sample in &selected_samples {
            assert_eq!(
                sample.edges.len(),
                sample.edge_count * 4,
                "sample edge count does not match edge buffer length for {}",
                sample.tile_key
            );

            let mut out = vec![0.0; sample.texture_size * sample.texture_size];
            let start = Instant::now();

            update_edge_distance_texture(
                black_box(&sample.edges),
                black_box(&mut out),
                black_box(sample.texture_size),
                black_box(sample.texture_size),
                black_box(sample.max_distance),
            );

            let duration = start.elapsed();
            total_duration += duration;
            checksum =
                checksum.wrapping_add(out.iter().map(|value| value.to_bits() as u64).sum::<u64>());

            println!(
                "repeat={:<4} tile={:<16} features={:<5} edges={:<7} tex={:<5} browser_ms={:<10.3} native_ms={:.3}",
                repeat_index + 1,
                sample.tile_key,
                sample.feature_count,
                sample.edge_count,
                sample.texture_size,
                sample.duration_ms,
                duration.as_secs_f64() * 1000.0,
            );
        }
    }

    println!(
        "done total_ms={:.3} measured_kernel_ms={:.3} checksum={}",
        profile_start.elapsed().as_secs_f64() * 1000.0,
        total_duration.as_secs_f64() * 1000.0,
        black_box(checksum),
    );
}

fn env_usize(name: &str) -> Option<usize> {
    std::env::var(name)
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
}
