use axum::{
    extract::MatchedPath,
    http::Request,
    middleware::{self, Next},
    response::IntoResponse,
    routing::get,
    Json, Router,
};

use hyper::{client::HttpConnector, Client, Uri};
use metrics_exporter_prometheus::{Matcher, PrometheusBuilder, PrometheusHandle};
use serde::{Deserialize, Serialize};
use std::{
    future::ready,
    net::SocketAddr,
    time::{Duration, Instant},
};
use uuid::Uuid;

mod handlers;
mod services;

#[tokio::main]
async fn main() {
    // Set the RUST_LOG, if it hasn't been explicitly defined
    // if std::env::var_os("RUST_LOG").is_none() {
    //     std::env::set_var("RUST_LOG", "example_todos=debug,tower_http=debug")
    // }
    tracing_subscriber::fmt::init();

    if let Err(e) = start().await {
        tracing::error!("failed to start application: {}", e);
        std::process::exit(1);
    }
}

async fn start() -> anyhow::Result<()> {
    let recorder_handler = setup_metrics_recorder()?;

    let ping = Router::new().route("/", get(|| async { "ok" }));
    let fast = Router::new()
        .route("/", get(fast))
        // fastだけ下記のレイヤーが適応される
        .layer(middleware::from_fn(layer_call));

    let slow = Router::new().route("/", get(slow));
    let metrics = Router::new().route("/", get(move || ready(recorder_handler.render())));

    let app = Router::new()
        .nest("/", ping)
        .nest("/fast", fast)
        .nest("/slow", slow)
        .nest("/metrics", metrics)
        // .nest("/postgres", postgres)
        // routing pathに、matchした場合にコールされる。
        .route_layer(middleware::from_fn(track_metrics));

    let addr = SocketAddr::from(([127, 0, 0, 1], 9000));
    tracing::debug!("listening on {}", addr);

    // loop
    tokio::spawn(async {
        tracing::info!("polling request.");
        polling_requests().await;
    });

    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await?;

    Ok(())
}

#[derive(Debug, Serialize, Clone)]
struct Message {
    id: Uuid,
    text: String,
}

async fn fast() -> impl IntoResponse {
    let message = Message {
        id: Uuid::new_v4(),
        text: "fast".into(),
    };
    Json(message)
    // (StatusCode::OK, Json(todo));
}

async fn slow() -> impl IntoResponse {
    tokio::time::sleep(Duration::from_secs(3)).await;
    "slow"
}

fn create_uri(uri: &Uri, path: &str) -> Uri {
    Uri::builder()
        .scheme(uri.scheme_str().unwrap_or("http"))
        .authority(uri.host().unwrap_or("127.0.0.1"))
        .path_and_query(path)
        .build()
        .expect("invalid uri.")
}

fn parse_uri() -> anyhow::Result<Uri> {
    let target_url = std::env::var("TARGET_URL")?;
    let base_uri: Uri = target_url.parse::<Uri>()?;
    base_uri
        .scheme_str()
        .ok_or_else(|| anyhow::anyhow!("target url must have scheme"))?;
    base_uri
        .host()
        .ok_or_else(|| anyhow::anyhow!("target url must have host"))?;
    Ok(base_uri)
}

async fn polling_requests() {
    let base_uri = match parse_uri() {
        Ok(uri) => uri,
        Err(e) => {
            tracing::error!("failed: {}", e);
            return;
        }
    };
    // create uri
    let weather: Uri = create_uri(&base_uri, "/weather");
    let lux: Uri = create_uri(&base_uri, "/lux");

    // request client
    let client = Client::builder()
        .pool_idle_timeout(Some(Duration::from_secs(30)))
        // .http2_only(true)
        .build_http();

    ////
    loop {
        let (weather_result, lux_result) = tokio::join!(
            fetch::<Weather>(&client, &weather),
            fetch::<Lux>(&client, &lux)
        );

        match weather_result {
            Ok(weather) => {
                tracing::debug!("{:?}", &weather);
                metrics::increment_counter!("weather_requests_success_total");
                metrics::gauge!("weather_humidity", weather.humidity);
                metrics::gauge!("weather_pressure", weather.pressure);
                metrics::gauge!("weather_temperature", weather.temp);
            }
            Err(e) => {
                tracing::error!("weather error: {}", e);
                metrics::increment_counter!("weather_requests_fail_total");
            }
        }
        match lux_result {
            Ok(lux) => {
                tracing::debug!("{:?}", &lux);
                metrics::increment_counter!("lux_requests_success_total");
                metrics::gauge!("lux_in_the_room ", lux.lux);
            }
            Err(e) => {
                tracing::error!("lux error: {}", e);
                metrics::increment_counter!("lux_requests_fail_total");
            }
        }

        tokio::time::sleep(Duration::from_secs(30)).await;
    }
}

async fn fetch<T: for<'de> Deserialize<'de>>(
    client: &hyper::Client<HttpConnector>,
    uri: &Uri,
) -> anyhow::Result<T> {
    let res = client.get(uri.to_owned()).await?;
    tracing::debug!("Response: {}", &res.status());

    if !res.status().is_success() {
        return Err(anyhow::anyhow!("failed to fetch {}", uri));
    }
    let body = hyper::body::to_bytes(res.into_body()).await?;
    let json: T = serde_json::from_slice(&body)?;
    Ok(json)
}

#[derive(Deserialize, Debug)]
struct Weather {
    humidity: f64,
    pressure: f64,
    temp: f64,
    // humidityUnit: String,
    // pressureUnit: String,
    // tempUnit: String,
    // timestamp: f64,
}

#[derive(Deserialize, Debug)]
struct Lux {
    lux: f64,
    // luxUnit: String,
    // timestamp: f64,
}

fn setup_metrics_recorder() -> anyhow::Result<PrometheusHandle> {
    const EXPONENTIAL_SECONDS: &[f64] = &[0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0];

    let handler = PrometheusBuilder::new()
        .set_quantiles(&[0.5, 0.9, 0.99])?
        .set_buckets_for_metric(
            Matcher::Full("http_requests_duration_seconds".to_string()),
            EXPONENTIAL_SECONDS,
        )?
        .install_recorder()?;
    Ok(handler)
}

async fn layer_call<B>(req: Request<B>, next: Next<B>) -> impl IntoResponse {
    tracing::info!("called....");
    let response = next.run(req).await;
    response
}

async fn track_metrics<B>(req: Request<B>, next: Next<B>) -> impl IntoResponse {
    tracing::info!("start !!!");

    let start = Instant::now();
    let path = if let Some(matched_path) = req.extensions().get::<MatchedPath>() {
        matched_path.as_str().to_owned()
    } else {
        req.uri().path().to_owned()
    };
    let method = req.method().clone().to_string();

    let response = next.run(req).await;

    let latency = start.elapsed().as_secs_f64();
    let status = response.status().as_u16().to_string();

    let labels = [("method", method), ("path", path), ("status", status)];

    metrics::increment_counter!("http_requests_total", &labels);
    metrics::histogram!("http_requests_duration_seconds", latency, &labels);
    response
}
