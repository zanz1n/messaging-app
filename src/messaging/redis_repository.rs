use super::repository::{Entry, MessagingConnection, MessagingRepository};
use crate::errors::ApiError;
use async_trait::async_trait;
use deadpool_redis::{
    redis::{aio::PubSub, AsyncCommands},
    Connection, Pool,
};
use tokio_stream::StreamExt;

pub struct RedisMessagingConnection {
    pub_sub: PubSub,
}

#[async_trait]
impl MessagingConnection for RedisMessagingConnection {
    async fn subscribe(&mut self, key: String) -> Result<(), ApiError> {
        self.pub_sub
            .subscribe(key)
            .await
            .or(Err(ApiError::MessagingSubscribeFailed))
    }

    async fn unsubscribe(&mut self, key: String) -> Result<(), ApiError> {
        self.pub_sub
            .unsubscribe(key)
            .await
            .or(Err(ApiError::MessagingUnsubscribeFailed))
    }

    async fn recv(&mut self) -> Result<Option<Entry>, ApiError> {
        let msg = self.pub_sub.on_message().next().await;

        Ok(match msg {
            Some(v) => Some(Entry {
                key: v.get_channel_name().to_string(),
                value: v.get_payload().or(Err(ApiError::MessagingRecvError))?,
            }),
            None => None,
        })
    }
}

pub struct RedisMessagingRepository {
    pool: Pool,
}

#[async_trait]
impl MessagingRepository<RedisMessagingConnection> for RedisMessagingRepository {
    async fn get_conn(&self) -> Result<RedisMessagingConnection, ApiError> {
        let conn = self
            .pool
            .get()
            .await
            .or(Err(ApiError::MessagingConnAcquireFailed))?;
        let pub_sub = Connection::take(conn).into_pubsub();

        Ok(RedisMessagingConnection { pub_sub })
    }

    async fn send(&self, data: Entry) -> Result<(), ApiError> {
        let mut conn = self
            .pool
            .get()
            .await
            .or(Err(ApiError::MessagingSendError))?;

        conn.publish(data.key, data.value)
            .await
            .or(Err(ApiError::MessagingSendError))
    }
}
