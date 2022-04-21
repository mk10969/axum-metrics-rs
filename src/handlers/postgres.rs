use std::time::Duration;

use axum::{routing::get, Extension, Router};
use sqlx::{postgres::PgPoolOptions, PgPool};

use crate::services::hello::{using_connection_extractor, using_connection_pool_extractor};

pub async fn postgres_handler() -> anyhow::Result<Router> {
    let db_connection_str = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:password@localhost".to_string());

    // setup connection pool
    let pool: PgPool = PgPoolOptions::new()
        .max_connections(5)
        .connect_timeout(Duration::from_secs(3))
        .connect(&db_connection_str)
        .await?;

    // build our application with some routes
    let postgres: Router = Router::new()
        .route(
            "/",
            get(using_connection_pool_extractor).post(using_connection_extractor),
        )
        .layer(Extension(pool));

    Ok(postgres)
}
