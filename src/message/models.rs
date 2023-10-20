use crate::http::ApiResponder;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Message {
    pub id: Uuid,
    pub user_id: Uuid,
    pub channel_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub content: Option<String>,
    pub image: Option<Uuid>,
}

impl ApiResponder for Message {
    fn unit() -> &'static str {
        "message"
    }
    fn article() -> &'static str {
        "A"
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MessageCreateData {
    pub content: Option<String>,
    pub image: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MessageUpdateData {
    pub content: Option<String>,
    pub image: Option<Uuid>,
}

#[derive(Debug, Clone)]
pub(super) enum MessageUpdateVariant {
    Content(String),
    Image(Uuid),
    ContentAndImage(Uuid, String),
    None,
}

impl Into<MessageUpdateVariant> for MessageUpdateData {
    fn into(self) -> MessageUpdateVariant {
        if self.content.is_some() && self.image.is_some() {
            MessageUpdateVariant::ContentAndImage(self.image.unwrap(), self.content.unwrap())
        } else if let Some(content) = self.content {
            MessageUpdateVariant::Content(content)
        } else if let Some(image) = self.image {
            MessageUpdateVariant::Image(image)
        } else {
            MessageUpdateVariant::None
        }
    }
}
