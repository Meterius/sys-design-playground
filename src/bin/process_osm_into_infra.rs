use futures::future::try_join_all;
use itertools::Itertools;
use osmpbf::Element;
use crossterm::{
    cursor::{Hide, MoveTo, Show},
    execute, queue,
    terminal::{Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen},
};
use tokio_postgres::NoTls;
use std::time::{Duration, Instant};
use std::io::Write;

const NUM_CONSUMERS: usize = 4;

#[derive(Debug)]
struct LocationMsg {
    id: i64,
    lon: f64,
    lat: f64,
    tags: String,
}

#[derive(Debug)]
struct ReportMsg {
    worker_idx: usize,
    count: u64,
    ts: Instant,
}

fn encode_tags<'a>(tags: impl Iterator<Item = (&'a str, &'a str)>) -> String {
    serde_json::to_string(&tags.sorted_by_key(|(key, _)| *key).collect_vec()).unwrap()
}

async fn consumer_task(
    idx: usize,
    rx: async_channel::Receiver<LocationMsg>,
    report_tx: async_channel::Sender<ReportMsg>,
) {
    let mut count: u64 = 0;

    let (client, connection) =
        tokio_postgres::connect("dbname=app_db host=localhost user=dev password=dev", NoTls)
            .await
            .unwrap();

    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("Connection error: {e}");
        }
    });

    let mut tick = tokio::time::interval(Duration::from_secs(3));

    loop {
        tokio::select! {
            el = rx.recv() => {
                let Ok(loc) = el else { break };
                let LocationMsg { id, lon, lat, tags } = loc;
                client
                    .query(
                        "INSERT INTO locations (id, latitude, longitude, tags)
VALUES
    ($1, $2, $3, $4) ON CONFLICT(id)
DO UPDATE SET (latitude, longitude, tags) = (EXCLUDED.latitude, EXCLUDED.longitude, EXCLUDED.tags);",
                        &[&(id.rem_euclid(i32::MAX as i64) as i32), &lat, &lon, &tags],
                    )
                    .await
                    .unwrap();

                count += 1;
            }
            _ = tick.tick() => {
                // Periodic per-worker report to the UI/main thread.
                let _ = report_tx
                    .send(ReportMsg {
                        worker_idx: idx,
                        count,
                        ts: Instant::now(),
                    })
                    .await;
            }
        }
    }

    // Final best-effort report so the dashboard doesn't get stuck at a stale value.
    let _ = report_tx
        .send(ReportMsg {
            worker_idx: idx,
            count,
            ts: Instant::now(),
        })
        .await;
}

async fn ui_task(report_rx: async_channel::Receiver<ReportMsg>) {
    let mut stdout = std::io::stdout();

    // Enter alternate screen + hide cursor so updates don't spam the console.
    let _ = execute!(stdout, EnterAlternateScreen, Hide, Clear(ClearType::All));

    // Header at row 0.
    let _ = queue!(
        stdout,
        MoveTo(0, 0),
        Clear(ClearType::CurrentLine),
        crossterm::style::Print("Worker   Count       TPS(5s)")
    );

    // Placeholder rows (1..=NUM_CONSUMERS).
    for idx in 0usize..NUM_CONSUMERS {
        let row = 1 + idx as u16;
        let line = format!("W{idx:02}   {:>10}   --", 0u64);
        let _ = queue!(
            stdout,
            MoveTo(0, row),
            Clear(ClearType::CurrentLine),
            crossterm::style::Print(line)
        );
    }

    let _ = stdout.flush();

    let mut last_count = vec![0u64; NUM_CONSUMERS];
    let mut last_ts: Vec<Option<Instant>> = vec![None; NUM_CONSUMERS];

    while let Ok(report) = report_rx.recv().await {
        let idx = report.worker_idx;
        let count = report.count;

        if idx >= NUM_CONSUMERS {
            continue;
        }

        let row = 1 + idx as u16;

        let tps_str = match last_ts[idx] {
            Some(prev_ts) => {
                let elapsed_secs = report.ts.duration_since(prev_ts).as_secs_f64();
                let delta = count.saturating_sub(last_count[idx]) as f64;
                if elapsed_secs > 0.0 {
                    let tps = delta / elapsed_secs;
                    last_count[idx] = count;
                    last_ts[idx] = Some(report.ts);
                    format!("{tps:>7.1}")
                } else {
                    // Extremely unlikely with `interval`, but avoid divide-by-zero.
                    last_count[idx] = count;
                    last_ts[idx] = Some(report.ts);
                    format!("{:>7}", "--")
                }
            }
            None => {
                last_count[idx] = count;
                last_ts[idx] = Some(report.ts);
                format!("{:>7}", "--")
            }
        };

        let line = format!("W{idx:02}   {count:>10}   {tps_str}");
        let _ = queue!(
            stdout,
            MoveTo(0, row),
            Clear(ClearType::CurrentLine),
            crossterm::style::Print(line)
        );
        let _ = stdout.flush();
    }

    // Restore terminal.
    let _ = execute!(stdout, Show, LeaveAlternateScreen);
}

fn producer_task(producer_tx: async_channel::Sender<LocationMsg>) {
    let reader =
        osmpbf::ElementReader::from_path("assets/datasets/osm/berlin-260406.osm.pbf")
            .unwrap();

    let total = reader
        .par_map_reduce(
            |el| {
                match el {
                    Element::Node(node) => {
                        producer_tx
                            .send_blocking(LocationMsg {
                                id: node.id(),
                                lon: node.lon(),
                                lat: node.lat(),
                                tags: encode_tags(node.tags()),
                            })
                            .unwrap();
                        1
                    },
                    Element::DenseNode(node) => {
                        producer_tx
                            .send_blocking(LocationMsg {
                                id: node.id(),
                                lon: node.lon(),
                                lat: node.lat(),
                                tags: encode_tags(node.tags()),
                            })
                            .unwrap();
                        1
                    },
                    _ => 0,
                }
            },
            || 0,
            |a, b| a + b,
        )
        .unwrap();

    // Use stderr so it doesn't interfere with the alternate-screen dashboard.
    eprintln!("Processed {total} elements");

    producer_tx.close();
}

#[tokio::main]
async fn main() {
    dotenvy::dotenv().unwrap();

    let (tx, rx) = async_channel::bounded::<LocationMsg>(4192);

    // Each worker periodically reports its latest count to the main/UI thread.
    // Workers do not need to coordinate with each other.
    let (report_tx, report_rx) = async_channel::unbounded::<ReportMsg>();

    let consumers = (0usize..NUM_CONSUMERS)
        .map(|idx| {
            let rx = rx.clone();
            let report_tx = report_tx.clone();
            tokio::task::spawn(consumer_task(idx, rx, report_tx))
        })
        .collect::<Vec<_>>();

    let ui_handle = tokio::task::spawn(ui_task(report_rx));

    let producer = std::thread::spawn({
        let producer_tx = tx.clone();
        move || producer_task(producer_tx)
    });

    tx.closed().await;
    producer.join().unwrap();
    try_join_all(consumers.into_iter()).await.unwrap();
    let _ = ui_handle.await;
}