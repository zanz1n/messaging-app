use super::{
    models::{Message, MessageCreateData, MessageUpdateData},
    repository::MessageRepository,
};
use crate::errors::ApiError;
use async_trait::async_trait;
use chrono::Utc;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::Mutex;
use uuid::Uuid;

#[derive(Default, Clone)]
pub struct InMemoryMessageRepository(Arc<Mutex<HashMap<Uuid, Message>>>);

#[async_trait]
impl MessageRepository for InMemoryMessageRepository {
    async fn get_by_id(&self, id: Uuid) -> Result<Option<Message>, ApiError> {
        let lock = self.0.lock().await;
        let msg = match lock.get(&id) {
            Some(v) => Some(v.clone()),
            None => None,
        };
        drop(lock);

        Ok(msg)
    }

    async fn get_many(
        &self,
        channel_id: Uuid,
        mut offset: u64,
        limit: u64,
    ) -> Result<Vec<Message>, ApiError> {
        let lock = self.0.lock().await;
        let mut arr = Vec::new();

        let mut i = 0u64;
        for (_, v) in lock.iter() {
            if offset > 0 {
                offset -= 1;
                continue;
            }
            if i > limit {
                break;
            }

            if v.channel_id == channel_id {
                arr.push(v.clone());
            }
            i += 1;
        }
        drop(lock);

        Ok(arr)
    }

    async fn create(
        &self,
        user_id: Uuid,
        channel_id: Uuid,
        data: MessageCreateData,
    ) -> Result<Message, ApiError> {
        let now = Utc::now();

        let msg = Message {
            id: Uuid::new_v4(),
            user_id,
            channel_id,
            content: data.content,
            created_at: now,
            updated_at: now,
            image: data.image,
        };

        let mut lock = self.0.lock().await;
        lock.insert(msg.id, msg.clone());
        drop(lock);

        Ok(msg)
    }

    async fn update(&self, id: Uuid, data: MessageUpdateData) -> Result<Message, ApiError> {
        let mut lock = self.0.lock().await;
        let msg = lock.get(&id);

        if let Some(v) = msg {
            let mut v = v.clone();

            if let Some(image) = data.image {
                v.image = Some(image);
            }
            if let Some(content) = data.content {
                v.content = Some(content);
            }
            lock.insert(id, v.clone());

            Ok(v)
        } else {
            Err(ApiError::MessageNotFound)
        }
    }

    async fn delete(&self, id: Uuid) -> Result<(), ApiError> {
        let mut lock = self.0.lock().await;
        let msg = lock.remove(&id);
        drop(lock);

        if msg.is_some() {
            Ok(())
        } else {
            Err(ApiError::MessageNotFound)
        }
    }
}
