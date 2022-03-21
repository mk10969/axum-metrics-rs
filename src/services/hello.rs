use axum::{
    async_trait,
    extract::{Extension, FromRequest, RequestParts},
};
use hyper::StatusCode;
use sqlx::PgPool;

// handler
pub async fn using_connection_pool_extractor(
    Extension(pool): Extension<PgPool>,
) -> Result<String, (StatusCode, String)> {
    tracing::info!("##### using_connection_pool_extractor");

    sqlx::query_scalar("select 'hello world from pg'")
        .fetch_one(&pool)
        .await
        .map_err(internal_error)
}

// handler
pub async fn using_connection_extractor(
    DatabaseConnection(conn): DatabaseConnection,
) -> Result<String, (StatusCode, String)> {
    tracing::info!("@@@@@ call using_connection_extractor");

    let mut conn = conn;
    sqlx::query_scalar("select 'hello world from pg'")
        .fetch_one(&mut conn)
        .await
        .map_err(internal_error)
}

// we can extract the connection pool with `Extension`

// we can also write a custom extractor that grabs a connection from the pool
// which setup is appropriate depends on your application
pub struct DatabaseConnection(sqlx::pool::PoolConnection<sqlx::Postgres>);

#[async_trait]
impl<B> FromRequest<B> for DatabaseConnection
where
    B: Send,
{
    type Rejection = (StatusCode, String);

    async fn from_request(req: &mut RequestParts<B>) -> Result<Self, Self::Rejection> {
        let Extension(pool) = Extension::<PgPool>::from_request(req)
            .await
            .map_err(internal_error)?;

        let conn = pool.acquire().await.map_err(internal_error)?;

        Ok(Self(conn))
    }
}

/// Utility function for mapping any error into a `500 Internal Server Error`
/// response.
fn internal_error<E>(err: E) -> (StatusCode, String)
where
    E: std::error::Error,
{
    (StatusCode::INTERNAL_SERVER_ERROR, err.to_string())
}
