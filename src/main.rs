use crate::{
    auth::handlers::AuthHandlers,
    channel::handlers::ChannelHandlers,
    gateway::handlers::ws_upgrader,
    http::AppData,
    message::handlers::MessageHandlers,
    setup::{env_param, JsonPanicHandler},
};
use axum::{routing, Extension, Router, Server};
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey};
use std::{error::Error, net::SocketAddr};
use tower_http::{catch_panic::CatchPanicLayer, normalize_path::NormalizePathLayer};
use tracing_subscriber::EnvFilter;

#[cfg(not(target_env = "msvc"))]
#[global_allocator]
static GLOBAL: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;

mod auth;
mod cache;
mod channel;
mod errors;
mod event;
mod gateway;
mod handlers;
mod http;
mod message;
mod setup;
mod user;

#[cfg(feature = "postgres")]
pub type UserRepo = crate::user::postgres_repository::PostgresUserRepository;
#[cfg(not(feature = "postgres"))]
pub type UserRepo = crate::user::memory_repository::InMemoryUserRepository;
#[cfg(feature = "postgres")]
pub type MessageRepo = crate::message::memory_repository::InMemoryMessageRepository;
#[cfg(not(feature = "postgres"))]
pub type MessageRepo = crate::message::memory_repository::InMemoryMessageRepository;
#[cfg(feature = "postgres")]
pub type ChannelRepo = crate::channel::memory_repository::InMemoryChannelRepository;
#[cfg(not(feature = "postgres"))]
pub type ChannelRepo = crate::channel::memory_repository::InMemoryChannelRepository;
#[cfg(feature = "redis")]
pub type CacheRepo = crate::cache::redis_repository::RedisCacheRepository;
#[cfg(not(feature = "redis"))]
pub type CacheRepo = crate::cache::memory_repository::InMemoryCacheRepository;
#[cfg(feature = "redis")]
pub type EventRepo = crate::event::redis_repository::RedisEventRepository;
#[cfg(not(feature = "redis"))]
pub type EventRepo = crate::event::memory_repository::InMemoryEventRepository;
pub type AuthRepo = crate::auth::jwt_repository::JwtAuthRepository<CacheRepo>;

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
            "/gateway",
            routing::get(ws_upgrader::<EventRepo, AuthRepo, ChannelRepo>),
        )
        .route(
            "/auth/signin",
            routing::post(handlers::post_auth_signin::<AuthRepo, UserRepo, EventRepo>),
        )
        .route(
            "/auth/signup",
            routing::post(handlers::post_auth_signup::<AuthRepo, UserRepo, EventRepo>),
        )
        .route(
            "/auth/self",
            routing::get(handlers::get_auth_self::<AuthRepo, UserRepo, EventRepo>),
        )
        .route(
            "/auth/self/invalidate",
            routing::post(handlers::post_auth_self_invalidate::<AuthRepo, UserRepo, EventRepo>),
        )
        .route(
            "/channel/:channel_id",
            routing::get(handlers::get_channel_id::<ChannelRepo, AuthRepo, EventRepo>),
        )
        .route(
            "/channels/self",
            routing::get(handlers::get_channels_self::<ChannelRepo, AuthRepo, EventRepo>),
        )
        .route(
            "/channel",
            routing::post(handlers::post_channel::<ChannelRepo, AuthRepo, EventRepo>),
        )
        .route(
            "/channel/:channel_id/permission",
            routing::put(handlers::put_channel_id_permission::<ChannelRepo, AuthRepo, EventRepo>),
        )
        .route(
            "/channel/:channel_id",
            routing::put(handlers::put_channel_id::<ChannelRepo, AuthRepo, EventRepo>),
        )
        .route(
            "/channel/:channel_id",
            routing::patch(handlers::put_channel_id::<ChannelRepo, AuthRepo, EventRepo>),
        )
        .route(
            "/channel/:channel_id",
            routing::delete(handlers::delete_channel_id::<ChannelRepo, AuthRepo, EventRepo>),
        )
        .route(
            "/channel/:channel_id/message/:message_id",
            routing::get(handlers::get_channel_id_message_id::<MessageRepo, ChannelRepo, AuthRepo, EventRepo>),
        )
        .route(
            "/channel/:channel_id/messages",
            routing::get(handlers::get_channel_id_messages::<MessageRepo, ChannelRepo, AuthRepo, EventRepo>),
        )
        .route(
            "/channel/:channel_id/message",
            routing::post(handlers::post_channel_id_message::<MessageRepo, ChannelRepo, AuthRepo, EventRepo>),
        )
        .route(
            "/channel/:channel_id/message/:message_id",
            routing::put(handlers::put_channel_id_message_id::<MessageRepo, ChannelRepo, AuthRepo, EventRepo>),
        )
        .route(
            "/channel/:channel_id/message/:message_id",
            routing::patch(
                handlers::put_channel_id_message_id::<MessageRepo, ChannelRepo, AuthRepo, EventRepo>,
            ),
        )
        .route(
            "/channel/:channel_id/message/:message_id",
            routing::delete(
                handlers::delete_channel_id_message_id::<MessageRepo, ChannelRepo, AuthRepo, EventRepo>,
            ),
        );

    #[cfg(feature = "postgres-redis-repository")]
    {
        use crate::{
            auth::jwt_repository::JwtAuthRepository, cache::redis_repository::RedisCacheRepository,
            event::redis_repository::RedisEventRepository,
            user::postgres_repository::PostgresUserRepository,
        };
        use deadpool_redis::{redis::cmd, Config, Connection, Runtime};
        use sqlx::postgres::PgPoolOptions;
        use std::time::{Duration, Instant};

        let jwt_token_duration = env_param("APP_JWT_DURATION").unwrap_or(3600_u64);
        let jwt_key = env_param::<String>("APP_JWT_KEY")?;
        let bcrypt_cost = env_param("APP_BCRYPT_COST").unwrap_or(bcrypt::DEFAULT_COST);
        let database_url = env_param::<String>("DATABASE_URL")?;
        let max_open_conns = env_param("DATABASE_MAX_CONNS").unwrap_or(12_u32);
        let min_open_conns = env_param("DATABASE_MIN_CONNS").unwrap_or(5_u32);
        let db_acquire_timeout = env_param("DATABASE_ACQUIRE_TIMEOUT").unwrap_or(8_u64);
        let redis_url = env_param::<String>("REDIS_URL")?;

        let redis_start = Instant::now();

        let redis_pool = Config::from_url(redis_url).create_pool(Some(Runtime::Tokio1))?;
        {
            let mut conn = redis_pool.get().await?;
            cmd("PING").query_async::<_, ()>(&mut conn).await?;
        }

        tracing::info!(
            took = format!("{}ms", (Instant::now() - redis_start).as_millis()),
            "Connected to redis"
        );

        let pg_start = Instant::now();

        let pool = PgPoolOptions::new()
            .after_connect(|conn, meta| {
                Box::pin(async move {
                    let version = conn.server_version_num();
                    tracing::info!(
                        pg_version = version,
                        age = format!("{}ms", meta.age.as_millis()),
                        idle_for = format!("{}ms", meta.idle_for.as_millis()),
                        "Opened postgres conn"
                    );
                    Ok(())
                })
            })
            .max_connections(max_open_conns)
            .min_connections(min_open_conns)
            .acquire_timeout(Duration::from_secs(db_acquire_timeout))
            .connect(&database_url)
            .await?;

        tracing::info!(
            took = format!("{}ms", (Instant::now() - pg_start).as_millis()),
            "Connected to postgres"
        );

        let user_repo = PostgresUserRepository::new(pool, bcrypt_cost);
        let cache_repo = RedisCacheRepository::new(redis_pool.clone());
        let auth_repo = JwtAuthRepository::new(
            Algorithm::HS512,
            EncodingKey::from_base64_secret(&jwt_key)?,
            DecodingKey::from_base64_secret(&jwt_key)?,
            jwt_token_duration,
            cache_repo,
        );
        let message_repo = MessageRepo::new();
        let channel_repo = ChannelRepo::new();
        let event_repo = RedisEventRepository::new(
            Connection::take(redis_pool.get().await?).into_pubsub(),
            redis_pool.get().await?,
        )
        .await?;

        let auth_handlers = AuthHandlers::new(auth_repo.clone(), user_repo, event_repo.clone());
        let message_handlers =
            MessageHandlers::new(message_repo, channel_repo.clone(), event_repo.clone());
        let channel_handlers = ChannelHandlers::new(channel_repo, event_repo.clone());

        app = app
            .layer(AppData::extension(auth_handlers))
            .layer(AppData::extension(message_handlers))
            .layer(AppData::extension(channel_handlers))
            .layer(AppData::extension(event_repo))
            .layer(Extension(auth_repo));
    }

    #[cfg(not(feature = "postgres-redis-repository"))]
    {
        use crate::{
            auth::jwt_repository::JwtAuthRepository,
            cache::memory_repository::InMemoryCacheRepository,
            channel::memory_repository::InMemoryChannelRepository,
            event::memory_repository::InMemoryEventRepository,
            message::memory_repository::InMemoryMessageRepository,
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
        let message_repo = InMemoryMessageRepository::new();
        let channel_repo = InMemoryChannelRepository::new();
        let event_repo = InMemoryEventRepository::new();

        let auth_handlers = AuthHandlers::new(auth_repo.clone(), user_repo, event_repo.clone());
        let message_handlers =
            MessageHandlers::new(message_repo, channel_repo.clone(), event_repo.clone());
        let channel_handlers = ChannelHandlers::new(channel_repo, event_repo.clone());

        app = app
            .layer(AppData::extension(auth_handlers))
            .layer(AppData::extension(message_handlers))
            .layer(AppData::extension(channel_handlers))
            .layer(AppData::extension(event_repo))
            .layer(Extension(auth_repo));
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
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("Failed building the tokio Runtime")
        .block_on(body())
}
