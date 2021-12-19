use storage::Storage;
use tokio::{sync::mpsc, task};

mod metrics;
mod storage;

fn scrape_endpoint() -> &'static str {
    "http://localhost:9100/metrics"
}

/// Fetches metrics from node_exporter
async fn fetch_metrics() -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let metric_payload = reqwest::get(scrape_endpoint()).await?.text().await?;
    let metrics = metric_payload
        .lines()
        .map(|s| s.to_string())
        .collect::<Vec<_>>();
    Ok(metrics)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (tx, mut rx) = mpsc::channel(32);

    let manager = tokio::spawn(async move {
        let mut storage = Storage::new();
        while let Some(cmd) = rx.recv().await {
            match storage.store(cmd) {
                Ok(_) => {
                    println!("Stored metric");
                }
                Err(err) => {
                    eprintln!("ERR: {}", err.to_string());
                }
            }
        }
    });

    let forever = task::spawn(async move {
        let mut interval_timer =
            tokio::time::interval(chrono::Duration::seconds(5).to_std().unwrap());
        loop {
            // Wait for the next interval tick
            interval_timer.tick().await;
            let tx = tx.clone();
            tokio::spawn(async move {
                let results = fetch_metrics().await.unwrap();
                for result in results {
                    if let Err(err) = tx.send(result).await {
                        eprintln!("Encountered Error {:?}", err);
                    }
                }
            });
        }
    });
    manager.await.unwrap();
    forever.await.unwrap();

    Ok(())
}
