use super::models::{Channel, ChannelCreateData, ChannelUpdateData, UserPermission};
use crate::errors::ApiError;
use async_trait::async_trait;
use uuid::Uuid;

#[async_trait]
pub trait ChannelRepository: Sync + Send {
    async fn get_by_id(&self, id: Uuid) -> Result<Option<Channel>, ApiError>;

    async fn get_by_user(
        &self,
        user_id: Uuid,
        offset: u64,
        limit: u64,
    ) -> Result<Vec<Channel>, ApiError>;

    async fn create(&self, data: ChannelCreateData) -> Result<Channel, ApiError>;

    async fn set_user_permission(
        &self,
        channel_id: Uuid,
        user_id: Uuid,
        perm: UserPermission,
    ) -> Result<(), ApiError>;

    async fn get_user_permisson(
        &self,
        user_id: Uuid,
        channel_id: Uuid,
    ) -> Result<UserPermission, ApiError>;

    async fn update(&self, id: Uuid, data: ChannelUpdateData) -> Result<Channel, ApiError>;

    async fn delete(&self, id: Uuid) -> Result<(), ApiError>;
}
