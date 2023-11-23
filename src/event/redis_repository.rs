use super::{
    models::AppEvent,
    repository::{EventConnection, EventRepository},
};
use crate::errors::ApiError;
use async_trait::async_trait;
use deadpool_redis::{
    redis::{aio::PubSub, AsyncCommands, RedisError},
    Connection,
};
use tokio::sync::broadcast::{error::RecvError, Receiver, Sender};
use tokio_stream::StreamExt;

const REDIS_CHANNEL: &'static str = "app_event";

pub struct RedisEventConnection {
    sub_recv: Receiver<AppEvent>,
}

#[async_trait]
impl EventConnection for RedisEventConnection {
    async fn recv(&mut self) -> Result<AppEvent, ApiError> {
        match self.sub_recv.recv().await {
            Ok(v) => Ok(v),
            Err(e) => {
                tracing::error!(error = e.to_string(), "Failed to receive event");
                Err(ApiError::MessagingRecvError)
            }
        }
    }
}

#[derive(Debug)]
pub struct RedisEventRepository {
    sub_sender: Sender<AppEvent>,
    pub_sender: Sender<AppEvent>,
}

impl RedisEventRepository {
    pub async fn new(
        mut recv_conn: PubSub,
        mut send_conn: Connection,
    ) -> Result<RedisEventRepository, RedisError> {
        match recv_conn.subscribe(REDIS_CHANNEL).await {
            Ok(v) => v,
            Err(e) => {
                tracing::error!(
                    error = e.to_string(),
                    "Failed to subscribe to app events redis channel"
                );
                return Err(e);
            }
        };

        let sub_sender = Sender::new(64);
        let pub_sender = Sender::new(64);

        let sub_sender_cl = sub_sender.clone();
        tokio::spawn(async move {
            let mut recv_stream = recv_conn.into_on_message();
            let sub_sender = sub_sender_cl;

            while let Some(msg) = recv_stream.next().await {
                if msg.get_channel_name() != REDIS_CHANNEL {
                    continue;
                }

                let payload = match msg.get_payload::<String>() {
                    Ok(v) => v,
                    Err(e) => {
                        tracing::error!(
                            error = e.to_string(),
                            "Failed to parse redis event to string"
                        );
                        continue;
                    }
                };

                let event = match serde_json::from_str(&payload) {
                    Ok(v) => v,
                    Err(e) => {
                        tracing::error!(error = e.to_string(), "Failed to parse redis event json");
                        continue;
                    }
                };

                match sub_sender.send(event) {
                    Ok(_) => {}
                    Err(e) => {
                        tracing::error!(
                            error = e.to_string(),
                            "Failed to send received event on memory channel"
                        );
                    }
                };
            }

            tracing::error!("Failed to receive redis event");
        });

        let mut pub_recv = pub_sender.subscribe();
        tokio::spawn(async move {
            loop {
                let event = match pub_recv.recv().await {
                    Ok(v) => v,
                    Err(e) => {
                        tracing::error!(
                            error = e.to_string(),
                            "Failed to receive and send queued event"
                        );
                        if e == RecvError::Closed {
                            break;
                        }
                        continue;
                    }
                };

                let event = match serde_json::to_string(&event) {
                    Ok(v) => v,
                    Err(e) => {
                        tracing::error!(error = e.to_string(), "Failed to serialize queued event");
                        continue;
                    }
                };

                match send_conn.publish(REDIS_CHANNEL, event).await {
                    Ok(v) => v,
                    Err(e) => {
                        tracing::error!(error = e.to_string(), "Failed to publish queued event");
                    }
                };
            }
        });

        Ok(RedisEventRepository {
            sub_sender,
            pub_sender,
        })
    }
}

#[async_trait]
impl EventRepository for RedisEventRepository {
    type Connection = RedisEventConnection;

    async fn get_conn(&self) -> Result<Self::Connection, ApiError> {
        Ok(RedisEventConnection {
            sub_recv: self.sub_sender.subscribe(),
        })
    }

    async fn publish(&self, event: AppEvent) -> Result<(), ApiError> {
        match self.pub_sender.send(event) {
            Ok(_) => Ok(()),
            Err(e) => {
                tracing::error!(error = e.to_string(), "Failed to publish event");
                Err(ApiError::MessagingSendError)
            }
        }
    }
}
