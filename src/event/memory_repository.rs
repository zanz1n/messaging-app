use super::{
    models::AppEvent,
    repository::{EventConnection, EventRepository},
};
use crate::errors::ApiError;
use async_trait::async_trait;
use tokio::sync::broadcast::{Receiver, Sender};

pub struct InMemoryEventConnection {
    receiver: Receiver<AppEvent>,
}

#[async_trait]
impl EventConnection for InMemoryEventConnection {
    async fn recv(&mut self) -> Result<AppEvent, ApiError> {
        match self.receiver.recv().await {
            Ok(v) => Ok(v),
            Err(e) => {
                tracing::error!(error = e.to_string(), "Failed to receive event");
                Err(ApiError::MessagingRecvError)
            }
        }
    }
}

#[derive(Clone)]
pub struct InMemoryEventRepository {
    sender: Sender<AppEvent>,
}

impl InMemoryEventRepository {
    pub fn new() -> Self {
        Self {
            sender: Sender::new(64),
        }
    }
}

#[async_trait]
impl EventRepository for InMemoryEventRepository {
    type Connection = InMemoryEventConnection;

    async fn get_conn(&self) -> Result<Self::Connection, ApiError> {
        Ok(InMemoryEventConnection {
            receiver: self.sender.subscribe(),
        })
    }

    async fn publish(&self, event: AppEvent) -> Result<(), ApiError> {
        match self.sender.send(event) {
            Ok(_) => Ok(()),
            Err(e) => {
                tracing::error!(error = e.to_string(), "Failed to publish event");
                Err(ApiError::MessagingSendError)
            }
        }
    }
}
