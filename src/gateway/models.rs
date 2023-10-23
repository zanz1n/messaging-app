use crate::{errors::ApiError, message::models::Message};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize)]
#[serde(
    tag = "type",
    content = "data",
    rename_all = "SCREAMING_SNAKE_CASE",
    deny_unknown_fields
)]
pub enum GatewayEvent {
    MessageCreated(Message),
    MessageUpdated(Message),
    MessageDelete { id: Uuid },
    Error(ApiError),
    Pong,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(
    tag = "type",
    content = "data",
    rename_all = "SCREAMING_SNAKE_CASE",
    deny_unknown_fields
)]
pub enum IncommingMessage {
    Ping,
}
