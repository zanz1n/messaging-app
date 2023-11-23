use crate::{auth::models::InvalidationReason, message::models::Message};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(
    tag = "type",
    content = "data",
    rename_all = "SCREAMING_SNAKE_CASE",
    deny_unknown_fields
)]
pub enum AppEvent {
    MessageCreated(Message),
    MessageUpdated(Message),
    ChannelDeleted(Uuid),
    ChannelCreated(Uuid),
    MessageDeleted { id: Uuid, channel: Uuid },
    UserInvalidated(Uuid, InvalidationReason),
}
