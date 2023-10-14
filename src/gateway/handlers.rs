use crate::{
    errors::ApiError,
    gateway::models::{GatewayMessage, IncommingMessage, MessagePayload},
    http::marshal_json_string,
};
use axum::{
    extract::{
        ws::{Message, WebSocket},
        WebSocketUpgrade,
    },
    response::IntoResponse,
    Error,
};
use serde::Serialize;
use std::{
    net::{Ipv4Addr, SocketAddr, SocketAddrV4},
    time::{Duration, Instant},
};
use tokio::{sync::broadcast::channel, time::sleep};

pub async fn ws_upgrader(
    ws: WebSocketUpgrade,
    // ConnectInfo(addr): ConnectInfo<SocketAddr>,
) -> impl IntoResponse {
    let addr = SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 33333));
    ws.on_upgrade(move |socket| ws_handler(socket, addr))
}

async fn send_message<T: Serialize>(ws: &mut WebSocket, value: &T) -> Result<(), Error> {
    ws.send(Message::Text(marshal_json_string(value))).await
}

pub async fn ws_handler(mut socket: WebSocket, addr: SocketAddr) {
    const SOCKET_TIMEOUT: Duration = Duration::from_secs(10);
    const SOCKET_TICK_CHECK: Duration = Duration::from_secs(7);

    tracing::info!(addr = addr.to_string(), "Incomming gateway connection");

    let (sender, mut receiver) = channel(10);
    let mut last_ping = Instant::now();

    tokio::spawn(async move {
        let mut id = 0u64;
        loop {
            id += 1;
            sleep(Duration::from_secs(3)).await;

            let content = id.to_string();

            let res = sender
                .send(GatewayMessage::MessageCreated(MessagePayload {
                    id,
                    content,
                }))
                .map_err(|e| {
                    tracing::error!(
                        error = e.to_string(),
                        "Failed to send message on tokio channel"
                    );
                });

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
            let e = ApiError::WebsocketTimeout(SOCKET_TIMEOUT.as_secs());
            match send_message(&mut socket, &GatewayMessage::Error(e)).await {
                Ok(_) => break Ok(()),
                Err(e) => break Err(e),
            }
        }

        if let Some(message) = message {
            let s = match message.to_text() {
                Ok(s) => s,
                Err(_) => match send_message(
                    &mut socket,
                    &GatewayMessage::Error(ApiError::WebsocketMessageNonUTF8),
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
                    &GatewayMessage::Error(ApiError::WebsocketMessageDeserializationFailed(
                        e.to_string(),
                    )),
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
                    if let Err(e) = send_message(&mut socket, &GatewayMessage::Pong).await {
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
