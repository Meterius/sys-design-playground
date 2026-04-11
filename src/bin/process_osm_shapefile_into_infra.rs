use futures::stream::{self, StreamExt};
use generated_queries::queries::osm_roads_queries::{
    UpsertRoadStreamingTransferStmt, UpsertRoadsStreamingCommitStmt, UpsertRoadsStreamingEndStmt,
    upsert_road_streaming_transfer, upsert_roads_streaming_commit, upsert_roads_streaming_end,
    upsert_roads_streaming_start,
};
use glam::DVec2;
use jlh_sys_design_playground::geo::osm::layered::model::road::Road;
use postgis::ewkb::{AsEwkbLineString, EwkbWrite, LineString, Point};
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::time::{Duration, Instant};
use tokio_postgres::binary_copy::BinaryCopyInWriter;
use tokio_postgres::types::Type;
use tokio_postgres::{Client, NoTls};
use tracing::{error, info};

const MAX_PARALLEL_DIRS: usize = 8;

#[derive(Default, Clone, Copy)]
struct DirStats {
    processed: u64,
    copied: u64,
    merged: u64,
}

fn linestring_to_ewkb(points: &[DVec2], srid: u32) -> Vec<u8> {
    let line = LineString {
        points: points
            .iter()
            .map(|p| Point {
                x: p.x,
                y: p.y,
                srid: None,
            })
            .collect(),
        srid: Some(srid as i32),
    };

    let mut out = Vec::new();
    if let Err(err) = line.as_ewkb().write_ewkb(&mut out) {
        panic!("failed to encode EWKB linestring: {err}");
    }
    out
}

fn collect_shapefile_dirs(root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let Ok(entries) = std::fs::read_dir(root) else {
        return out;
    };

    for entry in entries.flatten() {
        let dir_path = entry.path();
        if dir_path.is_dir() {
            out.push(dir_path);
        }
    }

    out.sort();
    out
}

struct RoadStreamingWriter<'a> {
    client: &'a Client,
    // Prepared for consistency with generated query flow; COPY still uses protocol API.
    _transfer_stmt: UpsertRoadStreamingTransferStmt,
    commit_stmt: UpsertRoadsStreamingCommitStmt,
    end_stmt: UpsertRoadsStreamingEndStmt,
    copy_writer: Pin<Box<BinaryCopyInWriter>>,
}

impl<'a> RoadStreamingWriter<'a> {
    async fn begin(client: &'a Client) -> Result<Self, tokio_postgres::Error> {
        let (start_stmt, transfer_stmt, commit_stmt, end_stmt) = tokio::try_join!(
            upsert_roads_streaming_start().prepare(client),
            upsert_road_streaming_transfer().prepare(client),
            upsert_roads_streaming_commit().prepare(client),
            upsert_roads_streaming_end().prepare(client),
        )?;
        start_stmt.bind(client).await?;

        // Use the exact COPY statement from queries/osm_roads_queries.sql.
        let copy_sink = client
            .copy_in(
                "COPY tmp_upsert_roads_streaming (
                    osm_id, reference, class, category, oneway, max_speed, layer,
                    is_bridge, is_tunnel, geom
                ) FROM stdin binary",
            )
            .await?;

        let copy_writer = Box::pin(BinaryCopyInWriter::new(
            copy_sink,
            &[
                Type::INT8,
                Type::TEXT,
                Type::TEXT,
                Type::TEXT,
                Type::TEXT,
                Type::INT4,
                Type::INT4,
                Type::BOOL,
                Type::BOOL,
                Type::BYTEA,
            ],
        ));

        Ok(Self {
            client,
            _transfer_stmt: transfer_stmt,
            commit_stmt,
            end_stmt,
            copy_writer,
        })
    }

    async fn write(&mut self, road: Road) -> Result<(), tokio_postgres::Error> {
        let geom_ewkb = linestring_to_ewkb(&road.geometry, 4326);
        let class = road.class.as_ref();
        let category_enum = road.class.category();
        let category = category_enum.as_ref();
        let oneway = match road.oneway {
            jlh_sys_design_playground::geo::osm::layered::model::road::OneWay::ForwardsOnly => {
                "forwards_only"
            }
            jlh_sys_design_playground::geo::osm::layered::model::road::OneWay::BackwardsOnly => {
                "backwards_only"
            }
            jlh_sys_design_playground::geo::osm::layered::model::road::OneWay::Bidirectional => {
                "bidirectional"
            }
        };
        let max_speed = road.max_speed.map(|v| v as i32);

        self.copy_writer
            .as_mut()
            .write(&[
                &road.osm_id,
                &road.reference,
                &class,
                &category,
                &oneway,
                &max_speed,
                &road.layer,
                &road.is_bridge,
                &road.is_tunnel,
                &geom_ewkb,
            ])
            .await
    }

    async fn finish(mut self) -> Result<(u64, u64), tokio_postgres::Error> {
        let copied = self.copy_writer.as_mut().finish().await?;
        let merged = self.commit_stmt.bind(self.client).await?;
        self.end_stmt.bind(self.client).await?;
        Ok((copied, merged))
    }
}

async fn process_shapefile_dir(
    client: &Client,
    dir: &Path,
) -> Result<DirStats, tokio_postgres::Error> {
    let shp_path = dir.join("gis_osm_roads_free_1.shp");
    if !shp_path.exists() {
        info!("skip_dir_no_target_shp dir={}", dir.display());
        return Ok(DirStats::default());
    }

    info!("processing_shapefile path={}", shp_path.display());
    let mut reader = match shapefile::reader::Reader::from_path(&shp_path) {
        Ok(reader) => reader,
        Err(err) => {
            error!("open_error path={} err={err:?}", shp_path.display());
            return Ok(DirStats::default());
        }
    };

    // Probe the first parsable road id and skip whole shapefile if it is already ingested.
    let first_osm_id = {
        let mut probe_reader = match shapefile::reader::Reader::from_path(&shp_path) {
            Ok(reader) => reader,
            Err(err) => {
                error!("probe_open_error path={} err={err:?}", shp_path.display());
                return Ok(DirStats::default());
            }
        };
        let mut found = None;
        for item in probe_reader.iter_shapes_and_records() {
            let Ok((shape, rec)) = item else {
                continue;
            };
            if let Ok(road) = Road::from_shapefile_item((&shape, &rec)) {
                found = Some(road.osm_id);
                break;
            }
        }
        found
    };

    if let Some(first_id) = first_osm_id {
        let exists = client
            .query_one(
                "SELECT EXISTS(SELECT 1 FROM osm_roads WHERE osm_id = $1)",
                &[&first_id],
            )
            .await?
            .get::<_, bool>(0);
        if exists {
            info!(
                "skip_shapefile_already_ingested path={} first_osm_id={}",
                shp_path.display(),
                first_id
            );
            return Ok(DirStats::default());
        }
    }

    let mut writer = RoadStreamingWriter::begin(client).await?;
    let started_at = Instant::now();
    let mut stats = DirStats::default();
    let mut window_count: u64 = 0;
    let mut last_tps_log = Instant::now();
    for item in reader.iter_shapes_and_records() {
        match item {
            Ok((shape, rec)) => match Road::from_shapefile_item((&shape, &rec)) {
                Ok(road) => {
                    writer.write(road).await?;
                    stats.processed += 1;
                    window_count += 1;

                    let elapsed = last_tps_log.elapsed();
                    if elapsed >= Duration::from_secs(5) {
                        let tps = (window_count as f64) / elapsed.as_secs_f64();
                        info!(
                            "ingest_tps={tps:.2} processed={} path={}",
                            stats.processed,
                            shp_path.display()
                        );
                        window_count = 0;
                        last_tps_log = Instant::now();
                    }
                }
                Err(err) => {
                    error!("Error={err:?} Rec={rec:?}");
                }
            },
            Err(err) => {
                error!("Error: {err:?}");
            }
        }
    }

    let (copied, merged) = writer.finish().await?;
    stats.copied = copied;
    stats.merged = merged;
    let elapsed_secs = started_at.elapsed().as_secs_f64();
    let avg_tps = if elapsed_secs > 0.0 {
        (stats.processed as f64) / elapsed_secs
    } else {
        0.0
    };
    info!(
        "shapefile_complete path={} processed={} copied={} merged={} avg_tps={avg_tps:.2}",
        shp_path.display(),
        stats.processed,
        stats.copied,
        stats.merged
    );
    Ok(stats)
}

async fn worker_task(worker_idx: usize, mut rx: tokio::sync::mpsc::Receiver<PathBuf>) -> DirStats {
    let (client, connection) =
        tokio_postgres::connect("dbname=app_db host=localhost user=dev password=dev", NoTls)
            .await
            .unwrap();
    let _connection_handle = tokio::spawn(async move {
        if let Err(err) = connection.await {
            error!("connection_error worker={} err={}", worker_idx, err);
        }
    });

    let mut totals = DirStats::default();
    while let Some(dir) = rx.recv().await {
        match process_shapefile_dir(&client, &dir).await {
            Ok(stats) => {
                totals.processed += stats.processed;
                totals.copied += stats.copied;
                totals.merged += stats.merged;
            }
            Err(err) => error!(
                "dir_processing_error worker={} dir={} err={:?}",
                worker_idx,
                dir.display(),
                err
            ),
        }
    }
    totals
}

#[tokio::main]
pub async fn main() {
    dotenvy::dotenv().unwrap();

    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .compact()
        .init();

    let shapefile_dirs = collect_shapefile_dirs(Path::new("./datasets/osm/shapefiles"));

    let worker_count = MAX_PARALLEL_DIRS.max(1);
    let mut senders = Vec::with_capacity(worker_count);
    let mut workers = Vec::with_capacity(worker_count);

    for idx in 0..worker_count {
        let (tx, rx) = tokio::sync::mpsc::channel::<PathBuf>(16);
        senders.push(tx);
        workers.push(tokio::spawn(worker_task(idx, rx)));
    }

    for (idx, dir) in shapefile_dirs.into_iter().enumerate() {
        let tx = &senders[idx % worker_count];
        let _ = tx.send(dir).await;
    }
    drop(senders);

    let mut totals = DirStats::default();
    let results = stream::iter(workers).then(|h| h).collect::<Vec<_>>().await;
    for result in results {
        match result {
            Ok(stats) => {
                totals.processed += stats.processed;
                totals.copied += stats.copied;
                totals.merged += stats.merged;
            }
            Err(err) => error!("worker_join_error err={}", err),
        }
    }

    info!("copy_complete rows={}", totals.copied);
    info!(
        "merge_complete rows={} processed={}",
        totals.merged, totals.processed
    );
}
