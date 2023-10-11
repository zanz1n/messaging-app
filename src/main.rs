mod errors;
mod http;
mod setup;

use crate::setup::{env_param, JsonPanicHandler};
use axum::{routing, Router, Server};
use std::{error::Error, net::SocketAddr};
use tower_http::{catch_panic::CatchPanicLayer, normalize_path::NormalizePathLayer};

pub type BoxedError = Box<dyn Error + Send + Sync>;

pub const ENCODING_FAILED_BODY: &[u8] =
    br#"{"message":"Failed to encode the response body","error_code":50000}"#;

async fn get_root() {}

#[tokio::main]
async fn main() -> Result<(), BoxedError> {
    #[cfg(feature = "dotenv")]
    {
        use crate::setup::VarError;
        dotenvy::dotenv().map_err(|_| VarError::DotenvFileNotFound)?;
    }
    tracing_subscriber::fmt::try_init()?;

    let port = env_param("APP_PORT").unwrap_or(8080_u16);

    let mut app = Router::new()
        .route("/", routing::get(get_root))
        .route("/", routing::post(get_root));

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

    server.serve(app.into_make_service()).await?;
    Ok(())
}
