mod auth;
mod cache;
mod channel;
mod errors;
mod gateway;
mod handlers;
mod http;
mod message;
mod messaging;
mod setup;
mod user;

#[cfg(feature = "postgres-redis-repository")]
mod impls {}

#[cfg(not(feature = "postgres-redis-repository"))]
mod impls {
    pub type UserRepo = crate::user::memory_repository::InMemoryUserRepository;
    pub type CacheRepo = crate::cache::memory_repository::InMemoryCacheRepository;
    pub type AuthRepo = crate::auth::jwt_repository::JwtAuthRepository<CacheRepo>;
    pub type MessageRepo = crate::message::memory_repository::InMemoryMessageRepository;
    pub type ChannelRepo = crate::channel::memory_repository::InMemoryChannelRepository;
}

use crate::{
    auth::handlers::AuthHandlers,
    http::AppData,
    impls::*,
    setup::{env_param, JsonPanicHandler},
};
use axum::{routing, Extension, Router, Server};
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey};
use std::{error::Error, net::SocketAddr};
use tower_http::{catch_panic::CatchPanicLayer, normalize_path::NormalizePathLayer};
use tracing_subscriber::EnvFilter;

pub type BoxedError = Box<dyn Error + Send + Sync>;

pub const ENCODING_FAILED_BODY: &[u8] =
    br#"{"message":"Failed to encode the response body","error_code":50000}"#;

async fn body() -> Result<(), BoxedError> {
    #[cfg(feature = "dotenv")]
    dotenvy::dotenv().map_err(|_| crate::setup::VarError::DotenvFileNotFound)?;

    #[cfg(feature = "json-log")]
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .json()
        .try_init()?;

    #[cfg(not(feature = "json-log"))]
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .try_init()?;

    let port = env_param("APP_PORT").unwrap_or(8080_u16);

    let mut app = Router::new();

    app = app
        .route(
            "/auth/signin",
            routing::post(handlers::post_auth_signin::<AuthRepo, UserRepo>),
        )
        .route(
            "/auth/signup",
            routing::post(handlers::post_auth_signup::<AuthRepo, UserRepo>),
        )
        .route(
            "/auth/self",
            routing::get(handlers::get_auth_self::<AuthRepo, UserRepo>),
        )
        .route(
            "/auth/self/invalidate",
            routing::post(handlers::post_auth_self_invalidate::<AuthRepo, UserRepo>),
        )
        .route(
            "/channel/{channel_id}/message/{message_id}",
            routing::get(handlers::get_channel_id_message_id::<MessageRepo, ChannelRepo, AuthRepo>),
        )
        .route(
            "/channel/{channel_id}/messages",
            routing::get(handlers::get_channel_id_messages::<MessageRepo, ChannelRepo, AuthRepo>),
        )
        .route(
            "/channel/{channel_id}/message",
            routing::post(handlers::post_channel_id_message::<MessageRepo, ChannelRepo, AuthRepo>),
        )
        .route(
            "/channel/{channel_id}/message/{message_id}",
            routing::put(handlers::put_channel_id_message_id::<MessageRepo, ChannelRepo, AuthRepo>),
        )
        .route(
            "/channel/{channel_id}/message/{message_id}",
            routing::patch(
                handlers::put_channel_id_message_id::<MessageRepo, ChannelRepo, AuthRepo>,
            ),
        );

    #[cfg(feature = "postgres-redis-repository")]
    {}

    #[cfg(not(feature = "postgres-redis-repository"))]
    {
        use crate::{
            auth::jwt_repository::JwtAuthRepository,
            cache::memory_repository::InMemoryCacheRepository,
            user::memory_repository::InMemoryUserRepository,
        };

        let jwt_token_duration = env_param("APP_JWT_DURATION").unwrap_or(3600_u64);
        let jwt_key = env_param::<String>("APP_JWT_KEY")?;
        let bcrypt_cost = env_param("APP_BCRYPT_COST").unwrap_or(bcrypt::DEFAULT_COST);

        let user_repo = InMemoryUserRepository::new(bcrypt_cost);
        let cache_repo = InMemoryCacheRepository::new();
        let auth_repo = JwtAuthRepository::new(
            Algorithm::HS512,
            EncodingKey::from_base64_secret(&jwt_key)?,
            DecodingKey::from_base64_secret(&jwt_key)?,
            jwt_token_duration,
            cache_repo,
        );

        let auth_handlers = AuthHandlers::new(auth_repo.clone(), user_repo);

        app = app
            .layer(AppData::extension(auth_handlers))
            .layer(Extension(auth_repo))
    }

    app = app
        .layer(NormalizePathLayer::trim_trailing_slash())
        .layer(CatchPanicLayer::custom(JsonPanicHandler));

    #[cfg(feature = "http-trace")]
    {
        app = app.layer(tower_http::trace::TraceLayer::new_for_http());
    }
    #[cfg(feature = "http-cors")]
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
