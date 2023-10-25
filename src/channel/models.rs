use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Channel {
    pub id: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChannelCreateData {
    pub name: String,
    pub init_users: Option<Vec<Uuid>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChannelUpdateData {
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum UserPermission {
    Owner,
    Admin,
    Interact,
    Read,
    None,
}

impl UserPermission {
    #[inline]
    pub fn can_delete_chan(&self) -> bool {
        match self {
            Self::Owner => true,
            _ => false,
        }
    }

    #[inline]
    pub fn can_update_chan(&self) -> bool {
        match self {
            Self::Owner | Self::Admin => true,
            _ => false,
        }
    }

    #[inline]
    pub fn can_delete_msg(&self) -> bool {
        match self {
            Self::Owner | Self::Admin => true,
            _ => false,
        }
    }

    #[inline]
    pub fn can_send_msg(&self) -> bool {
        match self {
            Self::Owner | Self::Admin | Self::Interact => true,
            _ => false,
        }
    }

    #[inline]
    pub fn can_read_msg(&self) -> bool {
        match self {
            Self::None => false,
            _ => true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UserPermissionEntry {
    pub channel_id: Uuid,
    pub user_id: Uuid,
    pub permission: UserPermission,
}
