use crate::{
    errors::ApiError,
    gateway::models::{GatewayEvent, IncommingMessage},
    http::marshal_json_string,
    message::models::Message,
};
use axum::{
    extract::{
        ws::{Message as WsMessage, WebSocket},
        ConnectInfo, WebSocketUpgrade,
    },
    response::IntoResponse,
    Error,
};
use chrono::Utc;
use serde::Serialize;
use std::{
    net::SocketAddr,
    time::{Duration, Instant},
};
use tokio::{sync::broadcast::channel, time::sleep};
use uuid::Uuid;

pub async fn ws_upgrader(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| ws_handler(socket, addr))
}

async fn send_message<T: Serialize>(ws: &mut WebSocket, value: &T) -> Result<(), Error> {
    ws.send(WsMessage::Text(marshal_json_string(value))).await
}

pub async fn ws_handler(mut socket: WebSocket, addr: SocketAddr) {
    const SOCKET_TIMEOUT: Duration = Duration::from_secs(30);
    const SOCKET_TICK_CHECK: Duration = Duration::from_secs(5);

    tracing::info!(addr = addr.to_string(), "Incomming gateway connection");

    let (sender, mut receiver) = channel::<GatewayEvent>(10);
    let mut last_ping = Instant::now();

    tokio::spawn(async move {
        let mut id = 0u64;
        loop {
            id += 1;
            sleep(Duration::from_secs(3)).await;

            let now = Utc::now();

            let res = sender.send(GatewayEvent::MessageCreated(Message {
                id: Uuid::new_v4(),
                channel_id: Uuid::new_v4(),
                user_id: Uuid::new_v4(),
                created_at: now,
                updated_at: now,
                content: Some(id.to_string()),
                image: None,
            }));

            if res.is_err() {
                break;
            }
        }
    });

    let res = loop {
        let message = tokio::select! {
            recv = socket.recv() => {
                match recv {
                    Some(result) => match result {
                        Err(e) => break Err(e),
                        Ok(msg) => Some(msg),
                    },
                    None => break Ok(()),
                }
            }
            recv = receiver.recv() => {
                match recv {
                    Ok(v) => {
                        if let Err(e) = send_message(&mut socket, &v).await {
                            break Err(e);
                        }
                    }
                    Err(e) => {
                        tracing::error!(
                            error = e.to_string(),
                            "Failed to receive message on tokio channel"
                        );
                    }
                }

                None
            }
            _ = sleep(SOCKET_TICK_CHECK) => {
                None
            }
        };

        if Instant::now() - last_ping > SOCKET_TIMEOUT {
            let e = ApiError::GatewayTimeout(SOCKET_TIMEOUT.as_secs());
            match send_message(&mut socket, &GatewayEvent::Error(e)).await {
                Ok(_) => break Ok(()),
                Err(e) => break Err(e),
            }
        }

        if let Some(message) = message {
            let s = match message.to_text() {
                Ok(s) => s,
                Err(_) => match send_message(
                    &mut socket,
                    &GatewayEvent::Error(ApiError::GatewayMessageNonUTF8),
                )
                .await
                {
                    Ok(_) => continue,
                    Err(e) => break Err(e),
                },
            };

            let data = match serde_json::from_str(s) {
                Ok(v) => v,
                Err(e) => match send_message(
                    &mut socket,
                    &GatewayEvent::Error(ApiError::GatewayDeserializationFailed(e.to_string())),
                )
                .await
                {
                    Ok(_) => continue,
                    Err(e) => break Err(e),
                },
            };

            match data {
                IncommingMessage::Ping => {
                    last_ping = Instant::now();
                    if let Err(e) = send_message(&mut socket, &GatewayEvent::Pong).await {
                        break Err(e);
                    }
                }
            }
        }
    };

    if let Err(e) = res {
        tracing::error!(
            error = e.to_string(),
            addr = addr.to_string(),
            "Connection closed unexpectedly"
        );
    }

    tracing::info!(addr = addr.to_string(), "Closed gateway connection");
}
