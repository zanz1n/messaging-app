use super::repository::CacheRepository;
use crate::errors::ApiError;
use async_trait::async_trait;
use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::sync::Mutex;

#[derive(Default, Clone)]
pub struct InMemoryCacheRepository {
    cache: Arc<Mutex<HashMap<String, String>>>,
    expiry: Arc<Mutex<HashMap<String, Instant>>>,
}

impl InMemoryCacheRepository {
    async fn background(self) {
        const INTERVAL: Duration = Duration::from_secs(2);

        let mut exclusion = Vec::new();
        loop {
            let now = Instant::now();

            let mut expiry = self.expiry.lock().await;

            for (k, v) in expiry.iter() {
                if now > *v {
                    exclusion.push(k.clone());
                }
            }

            if exclusion.len() != 0 {
                let mut cache = self.cache.lock().await;
                for e in exclusion.iter() {
                    cache.remove(e);
                    expiry.remove(e);
                }
                drop(cache);
            }
            drop(expiry);

            exclusion.clear();

            tokio::time::sleep(INTERVAL).await;
        }
    }

    pub fn new() -> InMemoryCacheRepository {
        let cache = InMemoryCacheRepository::default();
        tokio::spawn(cache.clone().background());

        cache
    }
}

#[async_trait]
impl CacheRepository for InMemoryCacheRepository {
    async fn get<K: ToString + Send>(&self, key: K) -> Result<Option<String>, ApiError> {
        let lock = self.cache.lock().await;

        Ok(match lock.get(&key.to_string()) {
            Some(v) => Some(v.clone()),
            None => None,
        })
    }

    async fn get_ttl<K: ToString + Send>(
        &self,
        key: K,
        ttl: u64,
    ) -> Result<Option<String>, ApiError> {
        let key = key.to_string();

        match self.get(key.clone()).await? {
            Some(v) => {
                let mut lock = self.expiry.lock().await;
                lock.insert(key, Instant::now() + Duration::from_secs(ttl));
                drop(lock);

                Ok(Some(v))
            }
            None => Ok(None),
        }
    }

    async fn set<K: ToString + Send>(&self, key: K, value: String) -> Result<(), ApiError> {
        let mut lock = self.cache.lock().await;
        lock.insert(key.to_string(), value);

        Ok(())
    }

    async fn set_ttl<K: ToString + Send>(
        &self,
        key: K,
        value: String,
        ttl: u64,
    ) -> Result<(), ApiError> {
        let key = key.to_string();

        let mut lock = self.cache.lock().await;
        lock.insert(key.clone(), value);
        drop(lock);

        let mut lock = self.expiry.lock().await;
        lock.insert(key, Instant::now() + Duration::from_secs(ttl));
        drop(lock);

        Ok(())
    }

    async fn delete<K: ToString + Send>(&self, key: K) -> Result<(), ApiError> {
        let mut lock = self.expiry.lock().await;
        lock.remove(&key.to_string());
        drop(lock);

        Ok(())
    }
}
