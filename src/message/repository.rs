use super::models::{Message, MessageCreateData, MessageUpdateData};
use crate::errors::ApiError;
use async_trait::async_trait;
use uuid::Uuid;

#[async_trait]
pub trait MessageRepository: Sync + Send {
    async fn get_by_id(&self, id: Uuid) -> Result<Option<Message>, ApiError>;

    async fn get_many(
        &self,
        channel_id: Uuid,
        offset: u64,
        limit: u64,
    ) -> Result<Vec<Message>, ApiError>;

    async fn create(
        &self,
        user_id: Uuid,
        channel_id: Uuid,
        data: MessageCreateData,
    ) -> Result<Message, ApiError>;

    async fn update(&self, id: Uuid, data: MessageUpdateData) -> Result<Message, ApiError>;

    async fn delete(&self, id: Uuid) -> Result<(), ApiError>;
}
