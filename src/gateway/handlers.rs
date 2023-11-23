use crate::{
    auth::{http::AuthExtractor, models::UserAuthPayload, repository::AuthRepository},
    channel::{models::UserPermission, repository::ChannelRepository},
    errors::ApiError,
    event::{
        models::AppEvent,
        repository::{EventConnection, EventRepository},
    },
    gateway::models::{GatewayEvent, IncommingMessage},
    http::{marshal_json_string, AppData},
};
use axum::{
    extract::{
        ws::{Message as WsMessage, WebSocket},
        ConnectInfo, WebSocketUpgrade,
    },
    response::Response,
    Error,
};
use serde::Serialize;
use std::{
    net::SocketAddr,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::time::sleep;
use uuid::Uuid;

pub async fn ws_upgrader<E, A, C>(
    AuthExtractor(auth_payload, _): AuthExtractor<A>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    AppData(event_repo): AppData<E>,
    AppData(channel_repo): AppData<C>,
    ws: WebSocketUpgrade,
) -> Result<Response, ApiError>
where
    E: EventRepository + 'static,
    A: AuthRepository + 'static,
    C: ChannelRepository + 'static,
{
    let conn = event_repo.get_conn().await?;

    Ok(ws.on_upgrade(move |socket| ws_handler(socket, addr, conn, auth_payload, channel_repo)))
}

async fn send_message<T: Serialize>(ws: &mut WebSocket, value: &T) -> Result<(), Error> {
    ws.send(WsMessage::Text(marshal_json_string(value))).await
}

async fn can_read_incomming_msg<C: ChannelRepository>(
    channel_repo: impl AsRef<C>,
    user_id: Uuid,
    channel_id: Uuid,
) -> bool {
    channel_repo
        .as_ref()
        .get_user_permission(user_id, channel_id)
        .await
        .unwrap_or_else(|e| {
            tracing::error!(error = e.to_string(), "Failed to get user permission");
            UserPermission::None
        })
        .can_read_msg()
}

async fn send_event(ws: &mut WebSocket, value: &GatewayEvent) {
    _ = ws
        .send(WsMessage::Text(marshal_json_string(value)))
        .await
        .map_err(|e| tracing::error!(error = e.to_string(), "Failed to send message on websocket"));
}

pub async fn ws_handler<EC: EventConnection, C: ChannelRepository>(
    mut socket: WebSocket,
    addr: SocketAddr,
    mut conn: EC,
    auth_payload: UserAuthPayload,
    channel_repo: Arc<C>,
) {
    const SOCKET_TIMEOUT: Duration = Duration::from_secs(30);
    const SOCKET_TICK_CHECK: Duration = Duration::from_secs(5);

    tracing::info!(addr = addr.to_string(), "Incomming gateway connection");

    let mut last_ping = Instant::now();

    let res = loop {
        tokio::select! {
            recv = socket.recv() => {
                if let Some(result) = recv {
                    match result {
                        Ok(message) => {
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
                        },
                        Err(e) => break Err(e),
                    }
                }
            }
            event = conn.recv() => {
                match event {
                    Ok(event) => match event {
                        AppEvent::MessageCreated(msg) => {
                            if can_read_incomming_msg(&channel_repo, auth_payload.sub, msg.channel_id)
                                .await
                            {
                                send_event(&mut socket, &GatewayEvent::MessageCreated(msg)).await
                            }
                        }
                        AppEvent::MessageUpdated(msg) => {
                            if can_read_incomming_msg(&channel_repo, auth_payload.sub, msg.channel_id)
                                .await
                            {
                                send_event(&mut socket, &GatewayEvent::MessageUpdated(msg)).await
                            }
                        }
                        AppEvent::MessageDeleted { id, channel } => {
                            if can_read_incomming_msg(&channel_repo, auth_payload.sub, channel).await {
                                send_event(&mut socket, &GatewayEvent::MessageDelete { id }).await
                            }
                        }
                        AppEvent::UserInvalidated(id, reason) => {
                            if id == auth_payload.sub {
                                tracing::info!(
                                    user_id = id.to_string(),
                                    invalidation_reason = reason.to_string(),
                                    "User disconected due to invalidation"
                                );
                                send_event(
                                    &mut socket,
                                    &GatewayEvent::Error(ApiError::AuthUserInvalidated),
                                )
                                .await;
                            }
                        }
                    },
                    Err(e) => {
                        tracing::error!(
                            error = e.to_string(),
                            "Failed to receive message on tokio channel"
                        );
                    }
                };
            }
            _ = sleep(SOCKET_TICK_CHECK) => {}
        };

        if Instant::now() - last_ping > SOCKET_TIMEOUT {
            let e = ApiError::GatewayTimeout(SOCKET_TIMEOUT.as_secs());
            match send_message(&mut socket, &GatewayEvent::Error(e)).await {
                Ok(_) => break Ok(()),
                Err(e) => break Err(e),
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
