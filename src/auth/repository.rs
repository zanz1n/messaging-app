use super::models::{InvalidationReason, UserAuthPayload, UserInvalidationPayload};
use crate::errors::ApiError;
use async_trait::async_trait;
use uuid::Uuid;

#[async_trait]
pub trait AuthRepository {
    async fn auth_user(&self, token: String) -> Result<UserAuthPayload, ApiError>;

    async fn login_user(&self, email: String, password: String) -> Result<String, ApiError>;

    async fn get_refresh_token(&self, user_id: Uuid) -> Result<String, ApiError>;

    async fn refresh_session(&self, refresh_token: String) -> Result<String, ApiError>;

    async fn generate_token(
        &self,
        user_id: Uuid,
        username: String,
        email: String,
    ) -> Result<String, ApiError>;

    async fn is_invalidated(
        &self,
        user_id: Uuid,
    ) -> Result<Option<UserInvalidationPayload>, ApiError>;

    async fn add_invalidation(
        &self,
        user_id: Uuid,
        reason: InvalidationReason,
    ) -> Result<(), ApiError>;
}
