mod metrics;

fn scrape_endpoint() -> &'static str {
    "http://localhost:9100/metrics"
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let metric_payload = reqwest::get(scrape_endpoint())
        .await?
        .text()
        .await?;
    let metrics = metric_payload
        .lines()
        .map(|line| metrics::Metric::from_str(line))
        .filter(|metric_or_none| metric_or_none.is_some())
        .map(|m| m.unwrap())
        .collect::<Vec<_>>();
    println!("{:?}", metrics);
    Ok(())
}