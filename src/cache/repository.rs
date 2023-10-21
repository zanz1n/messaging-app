use crate::errors::ApiError;
use async_trait::async_trait;
use serde::{de::DeserializeOwned, Serialize};

#[async_trait]
pub trait CacheRepository: Sync + Send {
    async fn get<K: ToString + Send>(&self, key: K) -> Result<Option<String>, ApiError>;

    async fn get_ttl<K: ToString + Send>(
        &self,
        key: K,
        ttl: u64,
    ) -> Result<Option<String>, ApiError>;

    async fn set<K: ToString + Send>(&self, key: K, value: String) -> Result<(), ApiError>;

    async fn set_ttl<K: ToString + Send>(
        &self,
        key: K,
        value: String,
        ttl: u64,
    ) -> Result<(), ApiError>;

    async fn delete<K: ToString + Send>(&self, key: K) -> Result<(), ApiError>;

    async fn de_get<T: DeserializeOwned>(&self, key: String) -> Result<Option<T>, ApiError> {
        let s = match self.get(key).await? {
            Some(v) => v,
            None => return Ok(None),
        };

        let t = serde_json::from_str(&s).map_err(|e| {
            tracing::error!(e = e.to_string(), "Failed to deserialize cache");
            ApiError::CacheDeserializationFailed
        })?;

        Ok(Some(t))
    }

    async fn de_get_ttl<T: DeserializeOwned, K: ToString + Send>(
        &self,
        key: K,
        ttl: u64,
    ) -> Result<Option<T>, ApiError> {
        let s = match self.get_ttl(key, ttl).await? {
            Some(v) => v,
            None => return Ok(None),
        };

        let t = serde_json::from_str(&s).map_err(|e| {
            tracing::error!(e = e.to_string(), "Failed to deserialize cache");
            ApiError::CacheDeserializationFailed
        })?;

        Ok(Some(t))
    }

    async fn ser_set<T: Serialize + Sync, K: ToString + Send>(
        &self,
        key: K,
        value: &T,
    ) -> Result<(), ApiError> {
        let v = match serde_json::to_string(value) {
            Ok(v) => v,
            Err(e) => {
                tracing::error!(e = e.to_string(), "Failed to serialize cache");

                return Err(ApiError::CacheSerializationFailed);
            }
        };

        self.set(key, v).await
    }

    async fn ser_set_ttl<T: Serialize + Sync, K: ToString + Send>(
        &self,
        key: K,
        value: &T,
        ttl: u64,
    ) -> Result<(), ApiError> {
        let v = match serde_json::to_string(value) {
            Ok(v) => v,
            Err(e) => {
                tracing::error!(e = e.to_string(), "Failed to serialize cache");

                return Err(ApiError::CacheSerializationFailed);
            }
        };

        self.set_ttl(key, v, ttl).await
    }
}
