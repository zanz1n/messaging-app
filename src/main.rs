mod auth;
mod cache;
mod errors;
mod gateway;
mod http;
mod message;
mod setup;
mod user;

use crate::{
    gateway::handlers::ws_upgrader,
    setup::{env_param, JsonPanicHandler},
};
use axum::{routing, Router, Server};
use std::{error::Error, net::SocketAddr};
use tower_http::{catch_panic::CatchPanicLayer, normalize_path::NormalizePathLayer};
use tracing_subscriber::EnvFilter;

pub type BoxedError = Box<dyn Error + Send + Sync>;

pub const ENCODING_FAILED_BODY: &[u8] =
    br#"{"message":"Failed to encode the response body","error_code":50000}"#;

async fn body() -> Result<(), BoxedError> {
    #[cfg(feature = "dotenv")]
    dotenvy::dotenv().map_err(|_| crate::setup::VarError::DotenvFileNotFound)?;

    #[cfg(feature = "json_log")]
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .json()
        .try_init()?;

    #[cfg(not(feature = "json_log"))]
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .try_init()?;

    let port = env_param("APP_PORT").unwrap_or(8080_u16);

    let mut app = Router::new().route("/gateway", routing::get(ws_upgrader));

    app = app
        .layer(NormalizePathLayer::trim_trailing_slash())
        .layer(CatchPanicLayer::custom(JsonPanicHandler));

    #[cfg(feature = "http_trace")]
    {
        app = app.layer(tower_http::trace::TraceLayer::new_for_http());
    }
    #[cfg(feature = "http_cors")]
    {
        use crate::setup::setup_app_cors;
        app = setup_app_cors(app);
    }

    let server = Server::try_bind(&SocketAddr::from(([0, 0, 0, 0], port)))?;
    tracing::info!(port, "Server listenning");

    server
        .serve(app.into_make_service_with_connect_info::<SocketAddr>())
        .await?;
    Ok(())
}

fn main() -> Result<(), BoxedError> {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("Failed building the Runtime")
        .block_on(body())
}
