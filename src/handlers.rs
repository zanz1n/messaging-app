use crate::{
    auth::{
        handlers::{AuthHandlers, InvalidationResponseBody, SignInRequestBody, SignInResponseBody},
        http::AuthExtractor,
        repository::AuthRepository,
    },
    errors::ApiError,
    http::{AppData, DataResponse, Json},
    user::{
        models::{User, UserCreateData},
        repository::UserRepository,
    },
};

pub async fn post_auth_signin<A, U>(
    AppData(data): AppData<AuthHandlers<A, U>>,
    Json(b): Json<SignInRequestBody>,
) -> Result<DataResponse<SignInResponseBody>, ApiError>
where
    A: AuthRepository + 'static,
    U: UserRepository + 'static,
{
    data.handle_signin(b).await
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
