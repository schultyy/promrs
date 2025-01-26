use std::{collections::HashMap, sync::Arc};

use clap::Parser;
use command::Command;
use opentelemetry::{
    trace::{TraceError, Tracer, TracerProvider as _},
    KeyValue,
};
use opentelemetry_otlp::{WithExportConfig, WithTonicConfig};
use opentelemetry_sdk::{
    self,
    trace::{self, RandomIdGenerator, Sampler, TracerProvider},
    Resource,
};
use storage::Storage;
use tokio::{
    sync::broadcast::{self, Receiver, Sender},
    task,
};
use tonic::metadata::{MetadataMap, MetadataValue};
use tracing::{debug, error, info, instrument, span, Level};
use tracing_subscriber::prelude::*;
use tracing_subscriber::{layer::SubscriberExt, Registry};
use warp::{reject, reply, Filter, Rejection, Reply};

mod command;
mod metrics;
mod storage;
mod storage_error;
mod web_error;

type WebResult<T> = std::result::Result<T, Rejection>;

fn scrape_endpoint() -> &'static str {
    "http://localhost:9100/metrics"
}

#[instrument]
async fn scrape_metrics() -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let endpoint = scrape_endpoint();
    info!("begin scrape for endpoint {}", endpoint);
    let metric_payload = reqwest::get(endpoint).await?.text().await?;
    let metrics = metric_payload
        .lines()
        .map(|s| s.to_string())
        .collect::<Vec<_>>();
    info!(
        "scraped {} metrics from endpoint {}",
        metrics.len(),
        endpoint
    );
    Ok(metrics)
}

fn init_tracer(endpoint: &str) -> Result<TracerProvider, TraceError> {
    let mut map = MetadataMap::with_capacity(3);

    map.insert("x-host", "example.com".parse().unwrap());
    map.insert("x-number", "123".parse().unwrap());
    map.insert_bin(
        "trace-proto-bin",
        MetadataValue::from_bytes(b"[binary data]"),
    );

    let otlp_exporter = opentelemetry_otlp::SpanExporter::builder()
        .with_tonic()
        .with_endpoint(endpoint)
        .with_metadata(map)
        .build()?;

    // Then pass it into provider builder
    let provider = opentelemetry_sdk::trace::TracerProvider::builder()
        .with_batch_exporter(otlp_exporter, opentelemetry_sdk::runtime::Tokio)
        .with_config(
            trace::Config::default()
                .with_sampler(Sampler::AlwaysOn)
                .with_id_generator(RandomIdGenerator::default())
                .with_max_events_per_span(64)
                .with_max_attributes_per_span(16)
                .with_max_events_per_span(16)
                .with_resource(Resource::new(vec![KeyValue::new(
                    "service.name",
                    "prom_rs",
                )])),
        )
        .build();
    Ok(provider)
}
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Prints traces to local stdout instead of jaeger
    #[arg(short, long)]
    local: bool,
    /// Otel endpoint
    #[arg(short, long, default_value_t = String::from("http://localhost:4317"))]
    otel_endpoint: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    if args.local {
        std::env::set_var("RUST_LOG", "INFO");
        tracing_subscriber::fmt::init();
    } else {
        let provider = init_tracer(&args.otel_endpoint).expect("Failed to initialize tracer");
        let tracer = provider.tracer("readme_example");
        let telemetry = tracing_opentelemetry::layer().with_tracer(tracer);
        let subscriber = Registry::default().with(telemetry);
        tracing::subscriber::with_default(subscriber, || {
            // Spans will be sent to the configured OpenTelemetry exporter
            let root = span!(tracing::Level::TRACE, "app_start", work_units = 2);
            let _enter = root.enter();

            error!("This event will be logged in the root span.");
        });
    }

    let vals: (Sender<Command>, Receiver<Command>) = broadcast::channel(500);
    let tx = vals.0;
    let mut rx1 = vals.1;
    let http_tx = tx.clone();
    let backchannel = tx.clone();

    let manager = tokio::spawn(async move {
        let mut storage = Storage::new();
        debug!("Spawn resource manager task");
        while let Ok(cmd) = rx1.recv().await {
            let span = span!(Level::TRACE, "manager_received_command");
            let _enter = span.enter();
            info!("received command {}", cmd.to_string());
            process_received_manager_command(cmd, &mut storage, &backchannel);
        }
    });

    let forever_fetch_metrics = task::spawn(async move {
        let mut interval_timer =
            tokio::time::interval(chrono::Duration::seconds(5).to_std().unwrap());
        loop {
            // Wait for the next interval tick
            interval_timer.tick().await;
            let tx = tx.clone();
            info!("Fetching Metrics");
            tokio::spawn(async move {
                let span = span!(Level::TRACE, "forever_scrape_metrics");
                let _enter = span.enter();
                let results = scrape_metrics().await.unwrap();
                if let Err(err) = tx.send(Command::Store(results)) {
                    eprintln!("Encountered Error {:?}", err);
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
            .and_then(handle_http_get_query)
            .recover(web_error::handle_rejection);

        info!("Listening at http://localhost:3030/query");

        warp::serve(query).run(([127, 0, 0, 1], 3030)).await;
    });

    manager.await.unwrap();
    forever_fetch_metrics.await.unwrap();
    forever_http_interface.await.unwrap();

    opentelemetry::global::shutdown_tracer_provider();
    Ok(())
}

fn process_received_manager_command(
    cmd: Command,
    storage: &mut Storage,
    backchannel: &Sender<Command>,
) {
    let span = span!(
        Level::TRACE,
        "process_received_manager_command",
        cmd = cmd.to_string().as_str()
    );
    let _enter = span.enter();
    info!(command = cmd.to_string().as_str(), "Received Command");
    process_command(cmd, storage, backchannel.clone());
}

#[instrument]
async fn handle_http_get_query(
    p: HashMap<String, String>,
    tx: Arc<Sender<Command>>,
) -> WebResult<impl Reply> {
    info!("handling query request");
    match p.get("key") {
        Some(query_str) => {
            info!("Querying Metric {}", query_str);
            if let Err(err) = tx.send(Command::Query(query_str.to_string())) {
                error!("{}", err);
                Err(reject::custom(web_error::Error::InternalServerError))
            } else {
                let mut rx = tx.subscribe();
                match rx.recv().await {
                    Ok(metrics) => {
                        info!("Received query results for {}", query_str);
                        format_reply(metrics)
                    }
                    Err(err) => {
                        error!("ERR while receiving metrics {}", err);
                        Err(reject::custom(web_error::Error::InternalServerError))
                    }
                }
            }
        }
        None => Err(reject::custom(web_error::Error::UnprocessablyEntity)),
    }
}

#[instrument]
fn format_reply(metrics: Command) -> WebResult<impl Reply> {
    match metrics {
        Command::QueryResults(metrics) => {
            info!("Returning {} results", metrics.len());
            let json_str = serde_json::to_string(&metrics).unwrap();
            Ok(reply::json(&json_str))
        }
        _ => {
            debug!(
                "Handling unexpected command message in `format_reply`: {}",
                metrics
            );
            Ok(reply::json(&"ok".to_string()))
        }
    }
}

fn process_command(cmd: Command, storage: &mut Storage, tx: Sender<Command>) {
    let span = span!(
        Level::TRACE,
        "process_command",
        cmd = cmd.to_string().as_str()
    );
    let _enter = span.enter();
    match cmd {
        Command::Store(cmd) => {
            store_data(storage, cmd);
        }
        Command::Query(query) => {
            fetch_data(storage, query, tx);
        }
        _ => {
            debug!("Trying to process unknown command {:?}", cmd);
        }
    }
}

#[instrument(skip(storage, tx))]
fn fetch_data(storage: &mut Storage, query: String, tx: Sender<Command>) {
    info!("fetching data from store for metric {}", query);
    let metrics = storage.query(query);
    if let Err(err) = tx.send(Command::QueryResults(metrics)) {
        error!("Error trying to send query results: {}", err);
    }
}

#[instrument(skip(storage, commands))]
fn store_data(storage: &mut Storage, commands: Vec<String>) {
    let span = span!(Level::TRACE, "store_data", commands = commands.len());
    let _enter = span.enter();
    info!("Storing batch of {} new commands", commands.len());
    for cmd in commands {
        match storage.store(cmd.to_string()) {
            Ok(()) => {
                info!(metric = cmd.to_string().as_str(), "Stored {}", cmd);
            }
            Err(err) => {
                debug!("{}", err.to_string());
            }
        }
    }
}
