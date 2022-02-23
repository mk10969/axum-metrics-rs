use axum::{
    extract::MatchedPath,
    http::{Request, StatusCode},
    response::IntoResponse,
    routing::get,
    Router,
};
use axum_extra::middleware::{self, Next};
use hyper::{Body, Client, Method, Response};
use metrics_exporter_prometheus::{Matcher, PrometheusBuilder, PrometheusHandle};
use std::{
    future::ready,
    net::SocketAddr,
    time::{Duration, Instant},
};

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

    let fast = Router::new()
        .route("/", get(fast))
        //　基本コールされる。しかし以下のパスにマッチするものはコールされない。。へー
        .layer(middleware::from_fn(layer_call));

    let slow = Router::new().route("/", get(slow));
    let metrics = Router::new().route("/", get(move || ready(recorder_handler.render())));
    let weather = Router::new().route("/", get(weather));

    let app = Router::new()
        .nest("/fast", fast)
        .nest("/slow", slow)
        .nest("/metrics", metrics)
        .nest("/weather", weather)
        // routing pathに、matchした場合にコールされる。
        .route_layer(middleware::from_fn(track_metrics));

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    tracing::debug!("listening on {}", addr);

    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await?;

    Ok(())
}

async fn fast() -> impl IntoResponse {
    tokio::time::sleep(Duration::from_millis(100)).await;
    "fast"
}

async fn slow() -> impl IntoResponse {
    tokio::time::sleep(Duration::from_secs(1)).await;
    "slow"
}

async fn weather() -> impl IntoResponse {
    // request
    let url = "http://100.64.1.151/weather".parse().unwrap();

    let client = Client::new();
    // Fetch the url...
    let res = client.get(url).await.unwrap();

    tracing::info!("Response: {}", &res.status());
    // asynchronously aggregate the chunks of the body
    // let body = hyper::body::aggregate(res).await.unwrap();

    let aa = hyper::body::to_bytes(res.into_body()).await.unwrap();

    tracing::info!("body: {}", String::from_utf8(aa.to_vec()).unwrap());

    (StatusCode::OK, "ok")
}

fn setup_metrics_recorder() -> anyhow::Result<PrometheusHandle> {
    const EXPONENTIAL_SECONDS: &[f64] = &[
        0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0,
    ];

    let handler = PrometheusBuilder::new()
        .set_quantiles(&[0.5, 0.9, 0.99])?
        .set_buckets_for_metric(
            Matcher::Full("http_requests_duration_seconds".to_string()),
            EXPONENTIAL_SECONDS,
        )?
        .install_recorder()?;
    Ok(handler)
}

async fn layer_call<B>(_req: Request<B>, _next: Next<B>) -> impl IntoResponse {
    tracing::info!("呼ばれたよ");
    ()
}
async fn track_metrics<B>(req: Request<B>, next: Next<B>) -> impl IntoResponse {
    tracing::info!("start!!!!!");

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
    metrics::increment_counter!("aaaaaaaaa", &labels);
    metrics::histogram!("bbbbbbbb", latency, &labels);

    response
}
