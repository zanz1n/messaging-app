use super::{
    models::{InvalidationReason, UserAuthPayload, UserInvalidationPayload},
    repository::AuthRepository,
};
use crate::{cache::repository::CacheRepository, errors::ApiError};
use async_trait::async_trait;
use base64::{engine::general_purpose, Engine};
use chrono::Utc;
use jsonwebtoken::{errors::ErrorKind, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use rand::Rng;
use tokio::task::spawn_blocking;
use uuid::Uuid;

#[derive(Clone)]
pub struct JwtAuthRepository<C: CacheRepository + Clone> {
    enc_key: EncodingKey,
    dec_key: DecodingKey,
    validation: Validation,
    algo: Algorithm,

    token_duration: u64,

    cache_repo: C,
}

impl<C> JwtAuthRepository<C>
where
    C: CacheRepository + Clone,
{
    pub fn new(
        algo: Algorithm,
        enc_key: EncodingKey,
        dec_key: DecodingKey,
        token_duration: u64,
        cache_repo: C,
    ) -> Self {
        let validation = Validation::new(algo);

        Self {
            enc_key,
            dec_key,
            validation,
            algo,
            token_duration,
            cache_repo,
        }
    }
}

#[async_trait]
impl<C> AuthRepository for JwtAuthRepository<C>
where
    C: CacheRepository + Clone,
{
    async fn auth_user(&self, token: String) -> Result<UserAuthPayload, ApiError> {
        let token = jsonwebtoken::decode(&token, &self.dec_key, &self.validation).map_err(|e| {
            match e.into_kind() {
                ErrorKind::ExpiredSignature => ApiError::AuthTokenExpired,
                _ => ApiError::AuthTokenInvalid,
            }
        })?;

        Ok(token.claims)
    }

    async fn login_user(
        &self,
        user_id: Uuid,
        username: String,
        user_email: String,
        user_password: String,
        password: String,
    ) -> Result<String, ApiError> {
        let b = spawn_blocking(move || bcrypt::verify(password, &user_password))
            .await
            .map_err(|e| {
                tracing::error!(error = e.to_string(), "Failed to spawn blocking");
                ApiError::AuthBcryptHashFailed
            })?
            .map_err(|e| {
                tracing::error!(
                    user_id = user_id.to_string(),
                    error = e.to_string(),
                    "Failed to compare user password hash"
                );
                ApiError::AuthBcryptHashFailed
            })?;

        if !b {
            return Err(ApiError::AuthFailed);
        }

        self.generate_token(user_id, username, user_email).await
    }

    async fn get_refresh_token(&self, user_id: Uuid) -> Result<String, ApiError> {
        let key = format!("refresh_token/{user_id}");

        let rt = self.cache_repo.get(&key).await?;
        let rt = match rt {
            Some(v) => v,
            None => {
                let value = generate_rf_token(user_id);
                self.cache_repo.set(key, value.clone()).await?;
                value
            }
        };

        Ok(rt)
    }

    async fn parse_refresh_token(&self, token: String) -> Result<Uuid, ApiError> {
        extract_rf_token_id(&token).ok_or(ApiError::AuthRefreshTokenInvalid)
    }

    async fn generate_token(
        &self,
        user_id: Uuid,
        username: String,
        email: String,
    ) -> Result<String, ApiError> {
        let claims = UserAuthPayload::new(user_id, username, email, self.token_duration);

        jsonwebtoken::encode(&Header::new(self.algo), &claims, &self.enc_key)
            .or(Err(ApiError::AuthTokenGenerationFailed))
    }

    async fn is_invalidated(
        &self,
        user_id: Uuid,
    ) -> Result<Option<UserInvalidationPayload>, ApiError> {
        let i = self
            .cache_repo
            .de_get(format!("user_invalidation/{user_id}"))
            .await?;

        Ok(i)
    }

    async fn add_invalidation(
        &self,
        user_id: Uuid,
        reason: InvalidationReason,
    ) -> Result<(), ApiError> {
        self.cache_repo
            .delete(format!("refresh_token/{user_id}"))
            .await?;

        let now = Utc::now();

        let value = UserInvalidationPayload {
            created_at: now,
            reason,
        };

        self.cache_repo
            .ser_set_ttl(
                format!("user_invalidation/{user_id}"),
                &value,
                self.token_duration + 10,
            )
            .await
    }
}

fn generate_rf_token(id: Uuid) -> String {
    let mut buf: [u8; 72] = [0; 72];
    let mut t_rng = rand::thread_rng();

    for b in &mut buf {
        *b = t_rng.gen();
    }

    let id = id.as_bytes();
    let mut i = 0;
    for b in id {
        buf[i] = *b;
        i += 1;
    }

    general_purpose::STANDARD.encode(buf)
}

fn extract_rf_token_id(s: &str) -> Option<Uuid> {
    let vec = match general_purpose::STANDARD.decode(s) {
        Ok(v) => v,
        Err(_) => return None,
    };

    if vec.len() != 72 {
        return None;
    }

    let (id, _) = vec.split_at(16);
    let id = match Uuid::from_slice(id) {
        Ok(v) => v,
        Err(_) => return None,
    };

    Some(id)
}

#[cfg(test)]
mod tests {
    use super::{extract_rf_token_id, generate_rf_token};
    use uuid::Uuid;

    #[test]
    fn test_generate_token() {
        let uuid = Uuid::new_v4();
        let token = generate_rf_token(uuid);

        match extract_rf_token_id(&token) {
            Some(v) => assert_eq!(v, uuid),
            None => panic!("Failed to extract id from generated token"),
        }
    }
}
