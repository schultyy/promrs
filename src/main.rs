use std::{collections::HashMap, fmt::Display, sync::Arc};

use log::{debug, error, info};
use tokio::{sync::{broadcast::{self, Receiver, Sender, error}}, task};
use warp::{Filter, Rejection, Reply, reject, reply};
use storage::Storage;

mod metrics;
mod storage;
mod storage_error;
mod web_error;

type WebResult<T> = std::result::Result<T, Rejection>;

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
    Store(String),
    Query(String),
    QueryResults(Vec<(i64, f64)>)
}

impl Display for Command {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Command::Store(metric) => {
                write!(f, "STORE {}", metric)
            },
            Command::Query(query) => {
                write!(f, "QUERY {}", query)
            },
            Command::QueryResults(metrics) => {
                write!(f, "QUERY RESULTS {:?}", metrics)
            },
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    let vals: (Sender<Command>, Receiver<Command>) = broadcast::channel(500);
    let tx = vals.0;
    let mut rx1 = vals.1;
    let http_tx = tx.clone();
    let backchannel = tx.clone();

    let manager = tokio::spawn(async move {
        let mut storage = Storage::new();
        debug!("Spawn resource manager task");
        while let Ok(cmd) = rx1.recv().await {
            info!("Received Command {:?}", cmd);
            process_command(cmd, &mut storage, backchannel.clone());
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
                    info!("Sending Metric \"{}\"", result);
                    if let Err(err) = tx.send(Command::Store(result)) {
                        eprintln!("Encountered Error {:?}", err);
                    }
                }
            });
        }
    });

    let forever_http_interface = task::spawn(async move {
        let tx = Arc::new(http_tx.clone());

        let tx = warp::any().map(move || Arc::clone(&tx));
        let query = warp::path!("query")
                .and(warp::query::<HashMap<String, String>>())
                .and(tx.clone())
                .and_then(get_query)
                .recover(web_error::handle_rejection);

        info!("Listening at http://localhost:3030/query");

        warp::serve(query)
            .run(([127, 0, 0, 1], 3030))
            .await;
    });

    manager.await.unwrap();
    forever_fetch_metrics.await.unwrap();
    forever_http_interface.await.unwrap();

    Ok(())
}

async fn get_query(p: HashMap<String, String>, tx: Arc<Sender<Command>>) -> WebResult<impl Reply> {
    info!("handling query request");
    match p.get("key") {
        Some(query_str) => {
            if let Err(err) = tx.send(Command::Query(query_str.to_string())) {
                error!("{}", err);
                Err(reject::custom(web_error::Error::InternalServerError))
            }
            else {
                let mut rx = tx.subscribe();
                match rx.recv().await {
                    Ok(metrics) => format_reply(metrics),
                    Err(err) => {
                        error!("ERR while receiving metrics {}", err);
                        Err(reject::custom(web_error::Error::InternalServerError))
                    }
                }
            }
        },
        None => Err(reject::custom(web_error::Error::UnprocessablyEntity))
    }
}

fn format_reply(metrics: Command) -> WebResult<impl Reply> {
    match metrics {
        Command::QueryResults(metrics) => {
            let json_str = serde_json::to_string(&metrics).unwrap();
            Ok(reply::json(&json_str))
        },
        _ => {
            debug!("Handling unexpected command message in `format_reply`: {}", metrics);
            Ok(reply::json(&"ok".to_string()))
        }
    }
}

fn process_command(cmd: Command, storage: &mut Storage, tx: Sender<Command>) {
    match cmd {
        Command::Store(cmd) => {
            store_data(storage, cmd);
        }
        Command::Query(query) => {
            fetch_data(storage, query, tx);
        },
        _ => {
            debug!("Trying to process unknown command {:?}", cmd);
        }
    }
}

fn fetch_data(storage: &mut Storage, query: String, tx: Sender<Command>) {
    let metrics = storage.query(query);
    if let Err(err) = tx.send(Command::QueryResults(metrics)) {
        error!("Error trying to send query results: {}", err);
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