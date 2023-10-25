use super::{
    models::{User, UserCreateData, UserRole, UserUpdateData},
    repository::UserRepository,
};
use crate::errors::ApiError;
use async_trait::async_trait;
use chrono::Utc;
use std::{collections::HashMap, sync::Arc};
use tokio::{sync::Mutex, task::spawn_blocking};
use uuid::Uuid;

#[derive(Clone)]
pub struct InMemoryUserRepository {
    map: Arc<Mutex<HashMap<Uuid, User>>>,
    bcrypt_cost: u32,
}

impl Default for InMemoryUserRepository {
    fn default() -> Self {
        Self {
            map: Default::default(),
            bcrypt_cost: bcrypt::DEFAULT_COST,
        }
    }
}

impl InMemoryUserRepository {
    #[inline]
    pub fn new(bcrypt_cost: u32) -> Self {
        Self {
            map: Arc::new(Mutex::new(HashMap::new())),
            bcrypt_cost,
        }
    }
}

#[async_trait]
impl UserRepository for InMemoryUserRepository {
    async fn get_by_id(&self, id: Uuid) -> Result<Option<User>, ApiError> {
        let lock = self.map.lock().await;

        let user = lock.get(&id);

        if let Some(user) = user {
            Ok(Some(user.clone()))
        } else {
            Ok(None)
        }
    }

    async fn get_by_email(&self, email: String) -> Result<Option<User>, ApiError> {
        let lock = self.map.lock().await;

        for (_, u) in lock.iter() {
            if u.email == email {
                return Ok(Some(u.clone()));
            }
        }
        drop(lock);

        Ok(None)
    }

    async fn auth_by_email(&self, email: String, password: String) -> Result<bool, ApiError> {
        let user = self
            .get_by_email(email)
            .await?
            .ok_or(ApiError::UserNotFound)?;

        Ok(user.password == password)
    }

    async fn create(&self, role: UserRole, data: UserCreateData) -> Result<User, ApiError> {
        let id = Uuid::new_v4();

        let lock = self.map.lock().await;
        if lock.get(&id).is_some() {
            return Err(ApiError::UserAlreadyExists);
        }
        drop(lock);

        if self.get_by_email(data.email.clone()).await?.is_some() {
            return Err(ApiError::UserAlreadyExists);
        }

        let now = Utc::now();
        let bcrypt_cost = self.bcrypt_cost;

        let password = spawn_blocking(move || bcrypt::hash(data.password, bcrypt_cost))
            .await
            .map_err(|e| {
                tracing::error!(error = e.to_string(), "Failed to spawn blocking");
                ApiError::AuthBcryptHashFailed
            })?
            .map_err(|e| {
                tracing::error!(
                    user_id = id.to_string(),
                    error = e.to_string(),
                    "Failed to hash password while creating user"
                );
                ApiError::AuthBcryptHashFailed
            })?;

        let user = User {
            id,
            created_at: now,
            updated_at: now,
            email: data.email,
            password,
            username: data.username,
            role,
        };

        let mut lock = self.map.lock().await;
        lock.insert(id, user.clone());
        drop(lock);

        Ok(user)
    }

    async fn update(&self, id: Uuid, data: UserUpdateData) -> Result<User, ApiError> {
        let mut lock = self.map.lock().await;

        let mut user = match lock.get(&id) {
            Some(u) => u.clone(),
            None => return Err(ApiError::UserNotFound),
        };

        if let Some(username) = data.username {
            user.username = username;
        }

        lock.insert(id, user.clone());

        Ok(user)
    }

    async fn delete(&self, id: Uuid) -> Result<(), ApiError> {
        let mut lock = self.map.lock().await;

        if lock.remove(&id).is_none() {
            return Err(ApiError::UserNotFound);
        }
        drop(lock);

        Ok(())
    }
}
