use super::repository::CacheRepository;
use crate::errors::ApiError;
use async_trait::async_trait;
use deadpool_redis::{
    redis::{AsyncCommands, Expiry},
    Connection, Pool,
};

#[derive(Clone)]
pub struct RedisCacheRepository {
    pool: Pool,
}

impl RedisCacheRepository {
    pub fn new(pool: Pool) -> RedisCacheRepository {
        RedisCacheRepository { pool }
    }

    async fn acquire_conn(&self) -> Result<Connection, ApiError> {
        match self.pool.get().await {
            Ok(v) => Ok(v),
            Err(e) => {
                tracing::error!(error = e.to_string(), "Failed to acquire redis connection");
                Err(ApiError::RedisError)
            }
        }
    }
}

#[async_trait]
impl CacheRepository for RedisCacheRepository {
    async fn get<K: ToString + Send>(&self, key: K) -> Result<Option<String>, ApiError> {
        let mut conn = self.acquire_conn().await?;
        let key = key.to_string();

        match conn.get(key).await {
            Ok(v) => Ok(Some(v)),
            Err(_) => Ok(None),
        }
    }

    async fn get_ttl<K: ToString + Send>(
        &self,
        key: K,
        ttl: u64,
    ) -> Result<Option<String>, ApiError> {
        let mut conn = self.acquire_conn().await?;
        let key = key.to_string();

        match conn.get_ex(key, Expiry::EX(ttl as usize)).await {
            Ok(v) => Ok(v),
            Err(_) => Ok(None),
        }
    }

    async fn set<K: ToString + Send>(&self, key: K, value: String) -> Result<(), ApiError> {
        let mut conn = self.acquire_conn().await?;
        let key = key.to_string();

        conn.set(key, value).await.map_err(|e| {
            tracing::error!(error = e.to_string(), operation = "SET", "Redis error");
            ApiError::RedisError
        })
    }

    async fn set_ttl<K: ToString + Send>(
        &self,
        key: K,
        value: String,
        ttl: u64,
    ) -> Result<(), ApiError> {
        let mut conn = self.acquire_conn().await?;
        let key = key.to_string();

        conn.set_ex(key, value, ttl as usize).await.map_err(|e| {
            tracing::error!(error = e.to_string(), operation = "SET", "Redis error");
            ApiError::RedisError
        })
    }

    async fn delete<K: ToString + Send>(&self, key: K) -> Result<(), ApiError> {
        let mut conn = self.acquire_conn().await?;
        let key = key.to_string();

        conn.del(key).await.map_err(|e| {
            tracing::error!(error = e.to_string(), operation = "SET", "Redis error");
            ApiError::RedisError
        })
    }
}
