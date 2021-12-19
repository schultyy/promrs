mod metrics;

fn scrape_endpoint() -> &'static str {
    "http://localhost:9100/metrics"
}

/// Fetches metrics from node_exporter and parses them
/// into structs
async fn fetch_metrics() -> Result<(), Box<dyn std::error::Error>> {
    let metric_payload = reqwest::get(scrape_endpoint()).await?.text().await?;
    let metrics = metric_payload
        .lines()
        .map(|line| metrics::Metric::from_str(line))
        .filter(|metric_or_none| metric_or_none.is_some())
        .map(|m| m.unwrap())
        .collect::<Vec<_>>();
    println!("{:?}", metrics);
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    //We want a tick every five seconds
    let mut interval_timer = tokio::time::interval(chrono::Duration::seconds(5).to_std().unwrap());
    loop {
        // Wait for the next interval tick
        interval_timer.tick().await;
        tokio::spawn(async {
            let result = fetch_metrics().await;
            if let Err(err) = result {
                eprintln!("ERR: {:?}", err);
            }
        });
    }
}
