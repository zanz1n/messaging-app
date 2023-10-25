use super::{
    models::{Channel, ChannelCreateData, ChannelUpdateData, UserPermission, UserPermissionEntry},
    repository::ChannelRepository,
};
use crate::errors::ApiError;
use async_trait::async_trait;
use chrono::Utc;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::Mutex;
use uuid::Uuid;

#[derive(Default, Clone)]
pub struct InMemoryChannelRepository {
    channel_map: Arc<Mutex<HashMap<Uuid, Channel>>>,
    perm_map: Arc<Mutex<Vec<UserPermissionEntry>>>,
}

impl InMemoryChannelRepository {
    #[inline]
    pub fn new() -> Self {
        Self {
            channel_map: Arc::new(Mutex::new(HashMap::new())),
            perm_map: Arc::new(Mutex::new(Vec::new())),
        }
    }
}

#[async_trait]
impl ChannelRepository for InMemoryChannelRepository {
    async fn get_by_id(&self, id: Uuid) -> Result<Option<Channel>, ApiError> {
        let lock = self.channel_map.lock().await;
        match lock.get(&id) {
            Some(v) => Ok(Some(v.clone())),
            None => Ok(None),
        }
    }

    async fn get_by_user(
        &self,
        user_id: Uuid,
        mut offset: u64,
        limit: u64,
    ) -> Result<Vec<Channel>, ApiError> {
        let lock = self.perm_map.lock().await;
        let mut channel_id_vec = Vec::new();

        let mut i = 0;
        for perm in lock.iter() {
            if offset > 0 {
                offset -= 1;
                continue;
            }
            if i > limit {
                break;
            }

            if perm.user_id == user_id {
                channel_id_vec.push(perm.channel_id);
                i += 1;
            }
        }
        drop(lock);

        let mut channel_vec = Vec::with_capacity(channel_id_vec.len());

        let lock = self.channel_map.lock().await;
        for (id, chan) in lock.iter() {
            if channel_id_vec.contains(id) {
                channel_vec.push(chan.clone());
            }
        }

        Ok(channel_vec)
    }

    async fn create(&self, data: ChannelCreateData) -> Result<Channel, ApiError> {
        let id = Uuid::new_v4();
        let now = Utc::now();
        let channel = Channel {
            id,
            created_at: now,
            updated_at: now,
            name: data.name,
        };

        let mut lock = self.channel_map.lock().await;
        lock.insert(id, channel.clone());
        drop(lock);

        Ok(channel)
    }

    async fn set_user_permission(
        &self,
        channel_id: Uuid,
        user_id: Uuid,
        perm: UserPermission,
    ) -> Result<(), ApiError> {
        let mut lock = self.perm_map.lock().await;
        let mut need_insert = true;
        for p in lock.iter_mut() {
            if p.channel_id == channel_id && p.user_id == user_id {
                *p = UserPermissionEntry {
                    channel_id,
                    user_id,
                    permission: perm.clone(),
                };
                need_insert = false;
                break;
            }
        }
        if need_insert {
            lock.push(UserPermissionEntry {
                channel_id,
                user_id,
                permission: perm,
            });
        }

        Ok(())
    }

    async fn get_user_permisson(
        &self,
        user_id: Uuid,
        channel_id: Uuid,
    ) -> Result<UserPermission, ApiError> {
        let lock = self.perm_map.lock().await;
        let mut perm = UserPermission::None;
        for p in lock.iter() {
            if p.user_id == user_id && p.channel_id == channel_id {
                perm = p.permission.clone();
                break;
            }
        }

        Ok(perm)
    }

    async fn update(&self, id: Uuid, data: ChannelUpdateData) -> Result<Channel, ApiError> {
        let mut lock = self.channel_map.lock().await;
        let mut chan = match lock.get(&id) {
            Some(v) => v,
            None => return Err(ApiError::ChannelNotFound),
        }
        .clone();

        chan.name = data.name;
        lock.insert(id, chan.clone());

        Ok(chan)
    }

    async fn delete(&self, id: Uuid) -> Result<(), ApiError> {
        let mut lock = self.channel_map.lock().await;
        if lock.remove(&id).is_none() {
            return Err(ApiError::ChannelNotFound);
        }
        drop(lock);

        let mut new_vec = Vec::new();
        let mut lock = self.perm_map.lock().await;
        for p in lock.iter() {
            if p.channel_id != id {
                new_vec.push(p.clone())
            }
        }
        *lock = new_vec;

        Ok(())
    }
}
