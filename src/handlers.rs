use crate::{
    auth::{
        handlers::{AuthHandlers, InvalidationResponseBody, SignInRequestBody, SignInResponseBody},
        http::AuthExtractor,
        repository::AuthRepository,
    },
    channel::repository::ChannelRepository,
    errors::ApiError,
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

pub async fn post_auth_signin<A, U>(
    AppData(data): AppData<AuthHandlers<A, U>>,
    Json(body): Json<SignInRequestBody>,
) -> Result<DataResponse<SignInResponseBody>, ApiError>
where
    A: AuthRepository + 'static,
    U: UserRepository + 'static,
{
    data.handle_signin(body).await
}

pub async fn post_auth_signup<A, U>(
    AppData(data): AppData<AuthHandlers<A, U>>,
    Json(b): Json<UserCreateData>,
) -> Result<DataResponse<User>, ApiError>
where
    A: AuthRepository + 'static,
    U: UserRepository + 'static,
{
    data.handle_signup(b).await
}

pub async fn get_auth_self<A, U>(
    AuthExtractor(auth, _): AuthExtractor<A>,
    AppData(data): AppData<AuthHandlers<A, U>>,
) -> Result<DataResponse<User>, ApiError>
where
    A: AuthRepository + 'static,
    U: UserRepository + 'static,
{
    data.handle_get_self(auth).await
}

pub async fn post_auth_self_invalidate<A, U>(
    AuthExtractor(auth, _): AuthExtractor<A>,
    AppData(data): AppData<AuthHandlers<A, U>>,
) -> Result<DataResponse<InvalidationResponseBody>, ApiError>
where
    A: AuthRepository + 'static,
    U: UserRepository + 'static,
{
    data.handle_invalidate(auth).await
}

pub async fn get_channel_id_message_id<M, C, A>(
    AuthExtractor(auth, _): AuthExtractor<A>,
    AppData(data): AppData<MessageHandlers<M, C, A>>,
    Path(path): Path<ChannelIdMessageIdPathParams>,
) -> Result<DataResponse<Message>, ApiError>
where
    M: MessageRepository + 'static,
    C: ChannelRepository + 'static,
    A: AuthRepository + 'static,
{
    data.handle_get_one(auth, path).await
}

pub async fn get_channel_id_messages<M, C, A>(
    AuthExtractor(auth, _): AuthExtractor<A>,
    AppData(data): AppData<MessageHandlers<M, C, A>>,
    Path(path): Path<ChannelIdPathParams>,
    Query(query): Query<GetManyQueryParams>,
) -> Result<DataResponse<Vec<Message>>, ApiError>
where
    M: MessageRepository + 'static,
    C: ChannelRepository + 'static,
    A: AuthRepository + 'static,
{
    data.handle_get_many(auth, path, query).await
}

pub async fn post_channel_id_message<M, C, A>(
    AuthExtractor(auth, _): AuthExtractor<A>,
    AppData(data): AppData<MessageHandlers<M, C, A>>,
    Path(path): Path<ChannelIdPathParams>,
    Json(body): Json<MessageCreateData>,
) -> Result<DataResponse<Message>, ApiError>
where
    M: MessageRepository + 'static,
    C: ChannelRepository + 'static,
    A: AuthRepository + 'static,
{
    data.handle_create(auth, path, body).await
}

pub async fn put_channel_id_message_id<M, C, A>(
    AuthExtractor(auth, _): AuthExtractor<A>,
    AppData(data): AppData<MessageHandlers<M, C, A>>,
    Path(path): Path<ChannelIdMessageIdPathParams>,
    Json(body): Json<MessageUpdateData>,
) -> Result<DataResponse<Message>, ApiError>
where
    M: MessageRepository + 'static,
    C: ChannelRepository + 'static,
    A: AuthRepository + 'static,
{
    data.handle_update(auth, path, body).await
}
