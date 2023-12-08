use crate::{
    auth::{
        handlers::{AuthHandlers, InvalidationResponseBody, SignInRequestBody, SignInResponseBody},
        http::AuthExtractor,
        repository::AuthRepository,
    },
    channel::{
        handlers::{AddPermissionRequestBody, ChannelHandlers},
        models::{Channel, ChannelCreateData, ChannelUpdateData, UserPermissionEntry},
        repository::ChannelRepository,
    },
    errors::ApiError,
    event::repository::EventRepository,
    http::{AppData, DataResponse, Json},
    message::{
        handlers::{
            ChannelIdMessageIdPathParams, ChannelIdPathParams, GetManyQueryParams, MessageHandlers,
        },
        models::{Message, MessageCreateData, MessageUpdateData},
        repository::MessageRepository,
    },
    user::{
        models::{User, UserCreateData},
        repository::UserRepository,
    },
};
use axum::extract::{Path, Query};

pub async fn post_auth_signin<A, U, E>(
    AppData(data): AppData<AuthHandlers<A, U, E>>,
    Json(body): Json<SignInRequestBody>,
) -> Result<DataResponse<SignInResponseBody>, ApiError>
where
    A: AuthRepository + 'static,
    U: UserRepository + 'static,
    E: EventRepository + 'static,
{
    data.handle_signin(body).await
}

pub async fn post_auth_signup<A, U, E>(
    AppData(data): AppData<AuthHandlers<A, U, E>>,
    Json(b): Json<UserCreateData>,
) -> Result<DataResponse<User>, ApiError>
where
    A: AuthRepository + 'static,
    U: UserRepository + 'static,
    E: EventRepository + 'static,
{
    data.handle_signup(b).await
}

pub async fn get_auth_self<A, U, E>(
    AuthExtractor(auth, _): AuthExtractor<A>,
    AppData(data): AppData<AuthHandlers<A, U, E>>,
) -> Result<DataResponse<User>, ApiError>
where
    A: AuthRepository + 'static,
    U: UserRepository + 'static,
    E: EventRepository + 'static,
{
    data.handle_get_self(auth).await
}

pub async fn post_auth_self_invalidate<A, U, E>(
    AuthExtractor(auth, _): AuthExtractor<A>,
    AppData(data): AppData<AuthHandlers<A, U, E>>,
) -> Result<DataResponse<InvalidationResponseBody>, ApiError>
where
    A: AuthRepository + 'static,
    U: UserRepository + 'static,
    E: EventRepository + 'static,
{
    data.handle_invalidate(auth).await
}

pub async fn get_channel_id<C, A, E>(
    AuthExtractor(auth, _): AuthExtractor<A>,
    AppData(data): AppData<ChannelHandlers<C, E>>,
    Path(path): Path<crate::channel::handlers::ChannelIdPathParams>,
) -> Result<DataResponse<Channel>, ApiError>
where
    C: ChannelRepository + 'static,
    A: AuthRepository + 'static,
    E: EventRepository + 'static,
{
    data.handle_get_one(auth, path).await
}

pub async fn get_channels_self<C, A, E>(
    AuthExtractor(auth, _): AuthExtractor<A>,
    AppData(data): AppData<ChannelHandlers<C, E>>,
    Query(query): Query<crate::channel::handlers::GetManyQueryParams>,
) -> Result<DataResponse<Vec<Channel>>, ApiError>
where
    C: ChannelRepository + 'static,
    A: AuthRepository + 'static,
    E: EventRepository + 'static,
{
    data.handle_get_many_self(auth, query).await
}

pub async fn post_channel<C, A, E>(
    AuthExtractor(auth, _): AuthExtractor<A>,
    AppData(data): AppData<ChannelHandlers<C, E>>,
    Json(body): Json<ChannelCreateData>,
) -> Result<DataResponse<Channel>, ApiError>
where
    C: ChannelRepository + 'static,
    A: AuthRepository + 'static,
    E: EventRepository + 'static,
{
    data.handle_create(auth, body).await
}

pub async fn put_channel_id_permission<C, A, E>(
    AuthExtractor(auth, _): AuthExtractor<A>,
    AppData(data): AppData<ChannelHandlers<C, E>>,
    Path(path): Path<crate::channel::handlers::ChannelIdPathParams>,
    Json(body): Json<AddPermissionRequestBody>,
) -> Result<DataResponse<UserPermissionEntry>, ApiError>
where
    C: ChannelRepository + 'static,
    A: AuthRepository + 'static,
    E: EventRepository + 'static,
{
    data.handle_edit_user_permission(auth, path, body).await
}

pub async fn put_channel_id<C, A, E>(
    AuthExtractor(auth, _): AuthExtractor<A>,
    AppData(data): AppData<ChannelHandlers<C, E>>,
    Path(path): Path<crate::channel::handlers::ChannelIdPathParams>,
    Json(body): Json<ChannelUpdateData>,
) -> Result<DataResponse<Channel>, ApiError>
where
    C: ChannelRepository + 'static,
    A: AuthRepository + 'static,
    E: EventRepository + 'static,
{
    data.handle_update(auth, path, body).await
}

pub async fn delete_channel_id<C, A, E>(
    AuthExtractor(auth, _): AuthExtractor<A>,
    AppData(data): AppData<ChannelHandlers<C, E>>,
    Path(path): Path<crate::channel::handlers::ChannelIdPathParams>,
) -> Result<DataResponse<()>, ApiError>
where
    C: ChannelRepository + 'static,
    A: AuthRepository + 'static,
    E: EventRepository + 'static,
{
    data.handle_delete(auth, path).await
}

pub async fn get_channel_id_message_id<M, C, A, E>(
    AuthExtractor(auth, _): AuthExtractor<A>,
    AppData(data): AppData<MessageHandlers<M, C, E>>,
    Path(path): Path<ChannelIdMessageIdPathParams>,
) -> Result<DataResponse<Message>, ApiError>
where
    M: MessageRepository + 'static,
    C: ChannelRepository + 'static,
    A: AuthRepository + 'static,
    E: EventRepository + 'static,
{
    data.handle_get_one(auth, path).await
}

pub async fn get_channel_id_messages<M, C, A, E>(
    AuthExtractor(auth, _): AuthExtractor<A>,
    AppData(data): AppData<MessageHandlers<M, C, E>>,
    Path(path): Path<ChannelIdPathParams>,
    Query(query): Query<GetManyQueryParams>,
) -> Result<DataResponse<Vec<Message>>, ApiError>
where
    M: MessageRepository + 'static,
    C: ChannelRepository + 'static,
    A: AuthRepository + 'static,
    E: EventRepository + 'static,
{
    data.handle_get_many(auth, path, query).await
}

pub async fn post_channel_id_message<M, C, A, E>(
    AuthExtractor(auth, _): AuthExtractor<A>,
    AppData(data): AppData<MessageHandlers<M, C, E>>,
    Path(path): Path<ChannelIdPathParams>,
    Json(body): Json<MessageCreateData>,
) -> Result<DataResponse<Message>, ApiError>
where
    M: MessageRepository + 'static,
    C: ChannelRepository + 'static,
    A: AuthRepository + 'static,
    E: EventRepository + 'static,
{
    data.handle_create(auth, path, body).await
}

pub async fn put_channel_id_message_id<M, C, A, E>(
    AuthExtractor(auth, _): AuthExtractor<A>,
    AppData(data): AppData<MessageHandlers<M, C, E>>,
    Path(path): Path<ChannelIdMessageIdPathParams>,
    Json(body): Json<MessageUpdateData>,
) -> Result<DataResponse<Message>, ApiError>
where
    M: MessageRepository + 'static,
    C: ChannelRepository + 'static,
    A: AuthRepository + 'static,
    E: EventRepository + 'static,
{
    data.handle_update(auth, path, body).await
}

pub async fn delete_channel_id_message_id<M, C, A, E>(
    AuthExtractor(auth, _): AuthExtractor<A>,
    AppData(data): AppData<MessageHandlers<M, C, E>>,
    Path(path): Path<ChannelIdMessageIdPathParams>,
) -> Result<DataResponse<()>, ApiError>
where
    M: MessageRepository + 'static,
    C: ChannelRepository + 'static,
    A: AuthRepository + 'static,
    E: EventRepository + 'static,
{
    data.handle_delete(auth, path).await
}
