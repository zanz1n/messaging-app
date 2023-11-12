use super::models::{User, UserCreateData, UserRole, UserUpdateData};
use crate::errors::ApiError;
use async_trait::async_trait;
use uuid::Uuid;

#[async_trait]
pub trait UserRepository: Sync + Send {
    async fn get_by_id(&self, id: Uuid) -> Result<Option<User>, ApiError>;
    async fn get_by_email(&self, email: String) -> Result<Option<User>, ApiError>;
    async fn create(&self, role: UserRole, data: UserCreateData) -> Result<User, ApiError>;
    async fn update(&self, id: Uuid, data: UserUpdateData) -> Result<User, ApiError>;
    async fn delete(&self, id: Uuid) -> Result<(), ApiError>;
}
