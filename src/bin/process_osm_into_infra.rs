use futures::future::try_join_all;
use osmpbf::Element;
use tokio_postgres::NoTls;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().unwrap();
    
    let (tx, rx) = async_channel::bounded::<(i64, f64, f64, String, String)>(4192);

    let consumers = (0..64)
        .map(|idx| {
            let rx = rx.clone();
            tokio::task::spawn(async move {
                let mut count = 0;

                let (client, connection) =
                    tokio_postgres::connect("dbname=app_db host=localhost user=dev password=dev", NoTls)
                        .await
                        .unwrap();

                tokio::spawn(async move {
                    if let Err(e) = connection.await {
                        eprintln!("Connection error: {e}");
                    }
                });

                while let Ok((id, lon, lat, name, desc)) = rx.recv().await {
                    client
                        .query(
                            "INSERT INTO locations (id, name, description, latitude, longitude)
VALUES
    ($1, $2, $3, $4, $5) ON CONFLICT(id)
DO UPDATE SET (name, description, latitude, longitude) = (EXCLUDED.name, EXCLUDED.description, EXCLUDED.latitude, EXCLUDED.longitude);",
                            &[&(id.rem_euclid(i32::MAX as i64) as i32), &name, &desc, &lat, &lon],
                        )
                        .await
                        .unwrap();

                    count += 1;

                    if count % 1000 == 0 {
                        println!("Worker {idx} processed {count} elements...");
                    }
                }
            })
        })
        .collect::<Vec<_>>();

    let reader =
        osmpbf::ElementReader::from_path("assets/datasets/osm/germany-latest.osm.pbf").unwrap();

    let producer_tx = tx.clone();
    let producer = std::thread::spawn(move || {
        let total = reader
            .par_map_reduce(
                |el| {
                    if let Element::DenseNode(node) = el
                        && let Some((_, name)) = node.tags().find(|(key, _)| key == &"name")
                    {
                        producer_tx
                            .send_blocking((
                                node.id,
                                node.lon(),
                                node.lat(),
                                name.to_owned(),
                                "".to_owned(),
                            ))
                            .unwrap();
                    }
                    1
                },
                || 0,
                |a, b| a + b,
            )
            .unwrap();

        println!("Processed {total} elements");

        producer_tx.close();
    });

    tx.closed().await;
    producer.join().unwrap();
    try_join_all(consumers.into_iter()).await.unwrap();
}
