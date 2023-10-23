use crate::http::ApiResponder;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE", deny_unknown_fields)]
pub enum UserRole {
    Admin,
    Common,
}

impl PartialEq for UserRole {
    fn eq(&self, other: &Self) -> bool {
        core::mem::discriminant(self) == core::mem::discriminant(other)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct User {
    pub id: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub email: String,
    pub username: String,
    pub role: UserRole,
    #[serde(skip_serializing)]
    pub password: String,
}

impl ApiResponder for User {
    fn unit() -> &'static str {
        "user"
    }
    fn article() -> &'static str {
        "An"
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UserCreateData {
    pub email: String,
    pub username: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UserUpdateData {
    pub username: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE", deny_unknown_fields)]
pub enum UserChannelPermission {
    Read,
    Write,
    None,
}

#[derive(Debug, Clone)]
pub(super) enum UserUpdateVariant {
    Username(String),
    None,
}

impl Into<UserUpdateVariant> for UserUpdateData {
    fn into(self) -> UserUpdateVariant {
        if let Some(username) = self.username {
            UserUpdateVariant::Username(username)
        } else {
            UserUpdateVariant::None
        }
    }
}
