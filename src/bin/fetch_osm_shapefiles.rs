use futures::stream::{self, StreamExt};
use geojson::Feature;
use jlh_sys_design_playground::geo::osm::client::fetch_fabrik_index;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use tracing::{info, warn};
use zip::ZipArchive;

const TARGET: &str = "germany";
const SHAPEFILES_ROOT: &str = "./datasets/osm/shapefiles";
const MAX_PARALLEL_DOWNLOADS: usize = 8;

#[derive(Clone)]
struct RegionNode {
    id: String,
    parent: Option<String>,
    shp_url: Option<String>,
}

fn prop_str<'a>(feature: &'a Feature, key: &str) -> Option<&'a str> {
    feature.property(key).and_then(|v| v.as_str())
}

fn shp_url(feature: &Feature) -> Option<String> {
    feature
        .property("urls")
        .and_then(|v| v.as_object())
        .and_then(|obj| obj.get("shp"))
        .and_then(|v| v.as_str())
        .map(ToOwned::to_owned)
}

fn build_nodes(features: &[Feature]) -> HashMap<String, RegionNode> {
    let mut nodes = HashMap::new();
    for feature in features {
        let Some(id) = prop_str(feature, "id") else {
            continue;
        };
        let parent = prop_str(feature, "parent").map(ToOwned::to_owned);
        nodes.insert(
            id.to_owned(),
            RegionNode {
                id: id.to_owned(),
                parent,
                shp_url: shp_url(feature),
            },
        );
    }
    nodes
}

#[allow(unused)]
fn descendants_including_target(nodes: &HashMap<String, RegionNode>, target: &str) -> Vec<String> {
    let mut children: HashMap<&str, Vec<&str>> = HashMap::new();
    for node in nodes.values() {
        if let Some(parent) = node.parent.as_deref() {
            children.entry(parent).or_default().push(node.id.as_str());
        }
    }

    let mut stack = vec![target];
    let mut seen = HashSet::new();
    let mut ordered = Vec::new();
    while let Some(id) = stack.pop() {
        if !seen.insert(id.to_owned()) {
            continue;
        }
        ordered.push(id.to_owned());
        if let Some(kids) = children.get(id) {
            for &child in kids {
                stack.push(child);
            }
        }
    }
    ordered
}

fn extract_zip_bytes(
    bytes: &[u8],
    out_dir: &Path,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let reader = Cursor::new(bytes);
    let mut archive = ZipArchive::new(reader)?;

    fs::create_dir_all(out_dir)?;
    for idx in 0..archive.len() {
        let mut file = archive.by_index(idx)?;
        let Some(safe_path) = file.enclosed_name().map(|p| p.to_path_buf()) else {
            continue;
        };
        let out_path = out_dir.join(safe_path);
        if file.name().ends_with('/') {
            fs::create_dir_all(&out_path)?;
            continue;
        }
        if let Some(parent) = out_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut out_file = fs::File::create(&out_path)?;
        std::io::copy(&mut file, &mut out_file)?;
    }

    Ok(())
}

async fn download_and_extract(client: reqwest::Client, id: String, url: String, out_dir: PathBuf) {
    info!("download id={} url={}", id, url);
    let bytes = match client.get(&url).send().await {
        Ok(resp) => match resp.error_for_status() {
            Ok(ok) => match ok.bytes().await {
                Ok(b) => b,
                Err(err) => {
                    warn!("skip id={} reason=read_body_error err={}", id, err);
                    return;
                }
            },
            Err(err) => {
                warn!("skip id={} reason=http_error err={}", id, err);
                return;
            }
        },
        Err(err) => {
            warn!("skip id={} reason=request_error err={}", id, err);
            return;
        }
    };

    let bytes = bytes.to_vec();
    let out_dir_clone = out_dir.clone();
    let res = tokio::task::spawn_blocking(move || extract_zip_bytes(&bytes, &out_dir_clone)).await;
    match res {
        Ok(Ok(())) => info!("extracted id={} path={}", id, out_dir.display()),
        Ok(Err(err)) => warn!("skip id={} reason=unzip_error err={}", id, err),
        Err(err) => warn!("skip id={} reason=join_error err={}", id, err),
    }
}

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .compact()
        .init();

    let client = reqwest::Client::new();
    let index = fetch_fabrik_index(&client)
        .await
        .expect("failed to fetch geofabrik index");
    let nodes = build_nodes(&index.features);
    let targets = descendants_including_target(&nodes, TARGET);

    if !nodes.contains_key(TARGET) {
        panic!("target `{TARGET}` not found in geofabrik index");
    }

    info!(
        "target={TARGET} descendants_including_target={}",
        targets.len()
    );
    let root = PathBuf::from(SHAPEFILES_ROOT);
    fs::create_dir_all(&root).expect("failed to create shapefiles root directory");

    let mut jobs: Vec<(String, String, PathBuf)> = Vec::new();
    for id in targets {
        let Some(node) = nodes.get(&id) else {
            continue;
        };
        let Some(url) = node.shp_url.as_deref() else {
            info!("skip id={} reason=no_urls_shp", id);
            continue;
        };

        let out_dir = root.join(&id);
        if out_dir.exists() {
            info!(
                "skip id={} reason=already_exists path={}",
                id,
                out_dir.display()
            );
            continue;
        }
        jobs.push((id, url.to_owned(), out_dir));
    }

    stream::iter(jobs)
        .map(|(id, url, out_dir)| {
            let client = client.clone();
            async move {
                download_and_extract(client, id, url, out_dir).await;
            }
        })
        .buffer_unordered(MAX_PARALLEL_DOWNLOADS)
        .collect::<Vec<_>>()
        .await;
}
