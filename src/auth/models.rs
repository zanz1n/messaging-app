use crate::user::models::User;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UserAuthPayload {
    pub sub: Uuid,
    pub email: String,
    pub exp: usize,
    pub iat: usize,
}

impl UserAuthPayload {
    #[inline]
    pub fn from_user(&self, user: &User, duration: usize) -> Self {
        Self::new(user.id, user.email.clone(), duration)
    }

    pub fn new(user_id: Uuid, email: String, duration: usize) -> Self {
        let now = Utc::now()
            .timestamp()
            .try_into()
            .expect("Failed to convert an unix timestamp integer type");

        Self {
            sub: user_id,
            email,
            exp: now + duration,
            iat: now,
        }
    }
}
