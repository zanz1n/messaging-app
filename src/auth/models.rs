use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UserAuthPayload {
    pub sub: Uuid,
    pub email: String,
    pub username: String,
    pub exp: u64,
    pub iat: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UserInvalidationPayload {
    #[serde(with = "chrono::serde::ts_seconds")]
    pub created_at: DateTime<Utc>,
    pub reason: InvalidationReason,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum InvalidationReason {
    Requested,
    PasswordChanged,
    Deleted,
}

impl UserAuthPayload {
    pub fn new(user_id: Uuid, username: String, email: String, duration: u64) -> Self {
        let now = Utc::now()
            .timestamp()
            .try_into()
            .expect("Failed to convert an unix timestamp integer type");

        Self {
            sub: user_id,
            email,
            username,
            exp: now + duration,
            iat: now,
        }
    }
}
