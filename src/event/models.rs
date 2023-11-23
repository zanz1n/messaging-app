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
    MessageDeleted { id: Uuid, channel_id: Uuid },
    ChannelDeleted(Uuid),
    ChannelUserAddedIn { id: Uuid, user_id: Uuid },
    ChannelUserRemovedFrom { id: Uuid, user_id: Uuid },
    UserInvalidated(Uuid, InvalidationReason),
}
