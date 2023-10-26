use super::{
    models::{Message, MessageCreateData, MessageUpdateData},
    repository::MessageRepository,
};
use crate::{
    auth::{models::UserAuthPayload, repository::AuthRepository},
    channel::repository::ChannelRepository,
    errors::ApiError,
    http::DataResponse,
};
use serde::Deserialize;
use std::marker::PhantomData;
use uuid::Uuid;

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

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChannelIdMessageIdPathParams {
    pub channel_id: Uuid,
    pub message_id: Uuid,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChannelIdPathParams {
    pub channel_id: Uuid,
}

pub struct MessageHandlers<M, C, A>
where
    M: MessageRepository,
    C: ChannelRepository,
    A: AuthRepository,
{
    message_repo: M,
    channel_repo: C,
    _pa: PhantomData<A>,
}

impl<M, C, A> MessageHandlers<M, C, A>
where
    M: MessageRepository,
    C: ChannelRepository,
    A: AuthRepository,
{
    pub async fn handle_get_one(
        &self,
        auth: UserAuthPayload,
        path: ChannelIdMessageIdPathParams,
    ) -> Result<DataResponse<Message>, ApiError> {
        let perm = self
            .channel_repo
            .get_user_permisson(auth.sub, path.channel_id)
            .await?;

        if !perm.can_read_msg() {
            return Err(ApiError::ChannelPermissionDenied);
        }

        let msg = match self.message_repo.get_by_id(path.message_id).await? {
            Some(v) => v,
            None => return Err(ApiError::MessageNotFound),
        };

        Ok(msg.into())
    }

    pub async fn handle_get_many(
        &self,
        auth: UserAuthPayload,
        path: ChannelIdPathParams,
        query: GetManyQueryParams,
    ) -> Result<DataResponse<Vec<Message>>, ApiError> {
        let perm = self
            .channel_repo
            .get_user_permisson(auth.sub, path.channel_id)
            .await?;

        if !perm.can_read_msg() {
            return Err(ApiError::ChannelPermissionDenied);
        }

        let msgs = self
            .message_repo
            .get_many(path.channel_id, query.offset, query.limit)
            .await?;

        Ok(msgs.into())
    }

    pub async fn handle_create(
        &self,
        auth: UserAuthPayload,
        path: ChannelIdPathParams,
        body: MessageCreateData,
    ) -> Result<DataResponse<Message>, ApiError> {
        let perm = self
            .channel_repo
            .get_user_permisson(auth.sub, path.channel_id)
            .await?;

        if !perm.can_send_msg() {
            return Err(ApiError::ChannelPermissionDenied);
        }

        let msg = self
            .message_repo
            .create(auth.sub, path.channel_id, body)
            .await?;

        Ok(msg.into())
    }

    pub async fn handle_update(
        &self,
        auth: UserAuthPayload,
        path: ChannelIdMessageIdPathParams,
        body: MessageUpdateData,
    ) -> Result<DataResponse<Message>, ApiError> {
        let perm = self
            .channel_repo
            .get_user_permisson(auth.sub, path.channel_id)
            .await?;

        if !perm.can_send_msg() {
            return Err(ApiError::ChannelPermissionDenied);
        }

        let msg = match self.message_repo.get_by_id(path.message_id).await? {
            Some(v) => v,
            None => return Err(ApiError::MessageNotFound),
        };

        if msg.user_id != auth.sub {
            return Err(ApiError::MessageEditDenied);
        }
        let msg = self.message_repo.update(msg.id, body).await?;

        Ok(msg.into())
    }
}
