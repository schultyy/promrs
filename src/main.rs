fn scrape_endpoint() -> &'static str {
    "http://localhost:9100/metrics"
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let metrics = reqwest::get(scrape_endpoint())
        .await?
        .text()
        .await?;
    println!("{:#?}", metrics);
    Ok(())
}