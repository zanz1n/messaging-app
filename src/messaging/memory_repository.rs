use super::repository::{Entry, MessagingConnection, MessagingRepository};
use crate::errors::ApiError;
use async_trait::async_trait;
use tokio::sync::broadcast::{Receiver, Sender};

pub struct InMemoryMessagingConnection {
    recv: Receiver<Entry>,
    sub: Vec<String>,
}

impl InMemoryMessagingConnection {
    #[inline]
    fn new(recv: Receiver<Entry>) -> Self {
        Self {
            recv,
            sub: Vec::new(),
        }
    }
}

#[async_trait]
impl MessagingConnection for InMemoryMessagingConnection {
    async fn subscribe(&mut self, key: String) -> Result<(), ApiError> {
        self.sub.push(key);

        Ok(())
    }

    async fn recv(&mut self) -> Result<Option<Entry>, ApiError> {
        match self.recv.recv().await {
            Ok(v) => {
                if self.sub.contains(&v.key) {
                    Ok(Some(v))
                } else {
                    drop(v);
                    return self.recv().await;
                }
            }
            Err(_) => Ok(None),
        }
    }
}

#[derive(Clone)]
pub struct InMemoryMessagingRepository(Sender<Entry>);

impl Default for InMemoryMessagingRepository {
    #[inline]
    fn default() -> Self {
        Self(Sender::new(24))
    }
}

impl InMemoryMessagingRepository {
    #[inline]
    pub fn new() -> InMemoryMessagingRepository {
        Default::default()
    }
}

#[async_trait]
impl MessagingRepository<InMemoryMessagingConnection> for InMemoryMessagingRepository {
    async fn get_conn(&self) -> Result<InMemoryMessagingConnection, ApiError> {
        Ok(InMemoryMessagingConnection::new(self.0.subscribe()))
    }

    async fn send(&self, data: Entry) -> Result<(), ApiError> {
        match self.0.send(data) {
            Ok(_) => Ok(()),
            Err(e) => {
                tracing::error!(error = e.to_string(), "Failed to send on tokio channel");
                Err(ApiError::MessagingSendError)
            }
        }
    }
}
