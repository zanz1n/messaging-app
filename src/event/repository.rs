use super::models::AppEvent;
use crate::errors::ApiError;
use async_trait::async_trait;

#[async_trait]
pub trait EventConnection {
    async fn recv(&mut self) -> Result<AppEvent, ApiError>;
}

#[async_trait]
pub trait EventRepository: Sync + Send {
    type Connection: EventConnection;

    async fn get_conn(&self) -> Result<Self::Connection, ApiError>;

    async fn publish(&self, event: AppEvent) -> Result<(), ApiError>;
}
