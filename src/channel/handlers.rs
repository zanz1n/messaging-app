use super::{
    models::{Channel, ChannelCreateData, ChannelUpdateData, UserPermission, UserPermissionEntry},
    repository::ChannelRepository,
};
use crate::{
    auth::models::UserAuthPayload,
    errors::ApiError,
    event::{models::AppEvent, repository::EventRepository},
    http::DataResponse,
};
use axum::http::StatusCode;
use serde::Deserialize;
use uuid::Uuid;

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChannelIdPathParams {
    pub channel_id: Uuid,
}

#[inline(always)]
fn default_limit() -> u64 {
    100
}
#[inline(always)]
fn default_offset() -> u64 {
    0
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GetManyQueryParams {
    #[serde(default = "default_limit")]
    pub limit: u64,
    #[serde(default = "default_offset")]
    pub offset: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE", deny_unknown_fields)]
pub enum AddPermissionVariant {
    Admin,
    Interact,
    Read,
    None,
}

impl Into<UserPermission> for AddPermissionVariant {
    fn into(self) -> UserPermission {
        match self {
            AddPermissionVariant::Admin => UserPermission::Admin,
            AddPermissionVariant::Interact => UserPermission::Interact,
            AddPermissionVariant::Read => UserPermission::Read,
            AddPermissionVariant::None => UserPermission::None,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AddPermissionRequestBody {
    user_id: Uuid,
    permission: AddPermissionVariant,
}

pub struct ChannelHandlers<C: ChannelRepository, E: EventRepository> {
    channel_repo: C,
    event_repo: E,
}

impl<C: ChannelRepository, E: EventRepository> ChannelHandlers<C, E> {
    pub fn new(channel_repo: C, event_repo: E) -> Self {
        Self {
            channel_repo,
            event_repo,
        }
    }

    pub async fn handle_get_one(
        &self,
        auth: UserAuthPayload,
        path: ChannelIdPathParams,
    ) -> Result<DataResponse<Channel>, ApiError> {
        let perm = self
            .channel_repo
            .get_user_permission(auth.sub, path.channel_id)
            .await?;

        if !perm.can_read_msg() {
            return Err(ApiError::ChannelPermissionDenied);
        }

        let chan = self
            .channel_repo
            .get_by_id(path.channel_id)
            .await?
            .ok_or(ApiError::ChannelNotFound)?;

        Ok(chan.into())
    }

    pub async fn handle_get_many_self(
        &self,
        auth: UserAuthPayload,
        query: GetManyQueryParams,
    ) -> Result<DataResponse<Vec<Channel>>, ApiError> {
        let chans = self
            .channel_repo
            .get_by_user(auth.sub, query.offset, query.limit)
            .await?;

        Ok(chans.into())
    }

    pub async fn handle_create(
        &self,
        auth: UserAuthPayload,
        body: ChannelCreateData,
    ) -> Result<DataResponse<Channel>, ApiError> {
        let chan = self.channel_repo.create(auth.sub, body.clone()).await?;

        if let Some(users) = body.init_users {
            for user_id in users {
                self.event_repo
                    .publish(AppEvent::ChannelUserAddedIn {
                        id: chan.id,
                        user_id,
                    })
                    .await?;
            }
        }

        Ok(chan.into())
    }

    pub async fn handle_edit_user_permission(
        &self,
        auth: UserAuthPayload,
        path: ChannelIdPathParams,
        body: AddPermissionRequestBody,
    ) -> Result<DataResponse<UserPermissionEntry>, ApiError> {
        let perm = self
            .channel_repo
            .get_user_permission(auth.sub, path.channel_id)
            .await?;

        if !perm.can_update_chan() {
            return Err(ApiError::ChannelPermissionDenied);
        }
        if body.permission == AddPermissionVariant::Admin && perm != UserPermission::Owner {
            return Err(ApiError::ChannelPermissionDenied);
        }

        let perm: UserPermission = body.permission.into();
        let before_permission = self
            .channel_repo
            .get_user_permission(body.user_id, path.channel_id)
            .await?;

        if before_permission != perm {
            self.channel_repo
                .set_user_permission(path.channel_id, body.user_id, perm.clone())
                .await?;

            if before_permission == UserPermission::None && perm != UserPermission::None {
                self.event_repo
                    .publish(AppEvent::ChannelUserAddedIn {
                        id: path.channel_id,
                        user_id: body.user_id,
                    })
                    .await?;
            } else if before_permission != UserPermission::None && perm == UserPermission::None {
                self.event_repo
                    .publish(AppEvent::ChannelUserRemovedFrom {
                        id: path.channel_id,
                        user_id: body.user_id,
                    })
                    .await?;
            }
        }

        Ok(UserPermissionEntry {
            channel_id: path.channel_id,
            permission: perm,
            user_id: body.user_id,
        }
        .into())
    }

    pub async fn handle_update(
        &self,
        auth: UserAuthPayload,
        path: ChannelIdPathParams,
        body: ChannelUpdateData,
    ) -> Result<DataResponse<Channel>, ApiError> {
        let perm = self
            .channel_repo
            .get_user_permission(auth.sub, path.channel_id)
            .await?;

        if !perm.can_update_chan() {
            return Err(ApiError::ChannelPermissionDenied);
        }
        let chan = self
            .channel_repo
            .update(path.channel_id, body.clone())
            .await?;

        self.event_repo
            .publish(AppEvent::ChannelUpdated(chan.id, body))
            .await?;

        Ok(chan.into())
    }

    pub async fn handle_delete(
        &self,
        auth: UserAuthPayload,
        path: ChannelIdPathParams,
    ) -> Result<DataResponse<()>, ApiError> {
        let perm = self
            .channel_repo
            .get_user_permission(auth.sub, path.channel_id)
            .await?;

        if !perm.can_delete_chan() {
            return Err(ApiError::ChannelPermissionDenied);
        }

        self.channel_repo.delete(path.channel_id).await?;

        _ = self
            .event_repo
            .publish(AppEvent::ChannelDeleted(path.channel_id))
            .await
            .map_err(|e| {
                tracing::error!(
                    error = e.to_string(),
                    channel_id = path.channel_id.to_string(),
                    "Faield to publish channel delete event"
                );
            });

        Ok(DataResponse {
            data: (),
            message: Some("Channel deleted".into()),
            http_code: Some(StatusCode::OK),
        })
    }
}
