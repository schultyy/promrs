use log::{error, info};
use tokio::{sync::mpsc, task};
use warp::Filter;

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

#[derive(Debug)]
enum Command {
    Store(String)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    let (tx, mut rx) = mpsc::channel(32);

    let manager = tokio::spawn(async move {
        let mut storage = Storage::new();
        while let Some(cmd) = rx.recv().await {
            process_command(cmd, &mut storage);
        }
    });

    let forever_fetch_metrics = task::spawn(async move {
        let mut interval_timer = tokio::time::interval(chrono::Duration::seconds(5).to_std().unwrap());
        loop {
            // Wait for the next interval tick
            interval_timer.tick().await;
            let tx = tx.clone();
            tokio::spawn(async move {
                let results = fetch_metrics().await.unwrap();
                for result in results {
                    if let Err(err) = tx.send(Command::Store(result)).await {
                        eprintln!("Encountered Error {:?}", err);
                    }
                }
            });
        }
    });

    let forever_http_interface = task::spawn(async move {
        let hello = warp::path!("hello" / String)
        .map(|name| format!("Hello, {}!", name));

        warp::serve(hello)
            .run(([127, 0, 0, 1], 3030))
            .await;
    });

    manager.await.unwrap();
    forever_fetch_metrics.await.unwrap();
    forever_http_interface.await.unwrap();

    Ok(())
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
            error!("{}", err.to_string());
        }
    }
}