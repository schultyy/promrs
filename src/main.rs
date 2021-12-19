use std::{collections::HashMap, convert::Infallible};

use log::{debug, error, info};
use tokio::{sync::{broadcast::{self, Receiver, Sender}, mpsc}, task};
use warp::{
    http::{Response, StatusCode},
    Filter,
};

use storage::Storage;

mod metrics;
mod storage;
mod storage_error;

fn scrape_endpoint() -> &'static str {
    "http://localhost:9100/metrics"
}

async fn fetch_metrics() -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let metric_payload = reqwest::get(scrape_endpoint())
        .await?
        .text()
        .await?;
    let metrics = metric_payload.lines()
        .map(|s| s.to_string())
        .collect::<Vec<_>>();
    Ok(metrics)
}

#[derive(Debug, Clone)]
enum Command {
    Store(String)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    // let (tx, mut rx) = mpsc::channel(32);
    let vals: (Sender<Command>, Receiver<Command>) = broadcast::channel(500);
    let tx = vals.0;
    let mut rx1 = vals.1;
    // let mut rx2 = tx.subscribe();

    let manager = tokio::spawn(async move {
        let mut storage = Storage::new();
        debug!("Spawn resource manager task");
        while let Ok(cmd) = rx1.recv().await {
            info!("Received Command {:?}", cmd);
            process_command(cmd, &mut storage);
        }
    });

    let forever_fetch_metrics = task::spawn(async move {
        let mut interval_timer = tokio::time::interval(chrono::Duration::seconds(5).to_std().unwrap());
        loop {
            // Wait for the next interval tick
            interval_timer.tick().await;
            let tx = tx.clone();
            info!("Fetching Metrics");
            tokio::spawn(async move {
                let results = fetch_metrics().await.unwrap();
                for result in results {
                    // info!("Sending Metric \"{}\"", result);
                    if let Err(err) = tx.send(Command::Store(result)) {
                        eprintln!("Encountered Error {:?}", err);
                    }
                }
            });
        }
    });

    let forever_http_interface = task::spawn(async move {
        let hello = warp::path!("query")
        .and(warp::query::<HashMap<String, String>>())
        .and_then(get_query);

        info!("Listening at http://localhost:3030/query");

        warp::serve(hello)
            .run(([127, 0, 0, 1], 3030))
            .await;
    });

    manager.await.unwrap();
    forever_fetch_metrics.await.unwrap();
    forever_http_interface.await.unwrap();

    Ok(())
}

async fn get_query(p: HashMap<String, String>) -> Result<impl warp::Reply, Infallible> {
    info!("handling query request");
    Ok(format!("I waited seconds!"))
    // match p.get("key") {
    //     Some(query_str) => {
    //         tx.send(Command::Query(query_str)).await;
    //         Response::builder().body(format!("key = {}", key))
    //     },
    //     None => Response::builder().body(String::from("No \"key\" param in query.")),
    // }
}

fn process_command(cmd: Command, storage: &mut Storage) {
    match cmd {
        Command::Store(cmd) => {
            store_data(storage, cmd);
        }
    }
}

fn store_data(storage: &mut Storage, cmd: String) {
    match storage.store(cmd.to_string()) {
        Ok(()) => {
            info!("Stored {}", cmd);
        }
        Err(err) => {
            debug!("{}", err.to_string());
        }
    }
}