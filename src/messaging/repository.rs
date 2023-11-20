use crate::errors::ApiError;
use async_trait::async_trait;
use serde::{de::DeserializeOwned, Serialize};

#[derive(Debug, Clone)]
pub struct Entry {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct SerdeEntry<T> {
    pub key: String,
    pub value: T,
}

#[async_trait]
pub trait MessagingConnection: Sync + Send {
    async fn subscribe(&mut self, key: String) -> Result<(), ApiError>;

    async fn unsubscribe(&mut self, key: String) -> Result<(), ApiError>;

    async fn recv(&mut self) -> Result<Option<Entry>, ApiError>;

    async fn de_recv<D: DeserializeOwned>(&mut self) -> Result<Option<SerdeEntry<D>>, ApiError> {
        let msg = match self.recv().await? {
            Some(v) => v,
            None => return Ok(None),
        };

        match serde_json::from_str(&msg.value) {
            Ok(value) => Ok(Some(SerdeEntry {
                key: msg.key,
                value,
            })),
            Err(e) => {
                tracing::error!(error = e.to_string(), "Failed to deserialize app event");

                Err(ApiError::MessagingDeserializationFailed)
            }
        }
    }
}

#[async_trait]
pub trait MessagingRepository<T: MessagingConnection>: Sync + Send {
    async fn get_conn(&self) -> Result<T, ApiError>;

    async fn send(&self, data: Entry) -> Result<(), ApiError>;

    async fn ser_send<D: Serialize + Send>(&self, data: SerdeEntry<D>) -> Result<(), ApiError> {
        let value = match serde_json::to_string(&data.value) {
            Ok(v) => v,
            Err(e) => {
                tracing::error!(error = e.to_string(), "Failed to serialize app event");
                return Err(ApiError::MessagingSerializationFailed);
            }
        };

        self.send(Entry {
            key: data.key,
            value,
        })
        .await
    }
}
