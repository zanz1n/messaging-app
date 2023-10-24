use super::{
    models::{InvalidationReason, UserAuthPayload},
    repository::AuthRepository,
};
use crate::{
    errors::ApiError,
    http::{ApiResponder, DataResponse},
    user::{
        models::{User, UserCreateData, UserRole},
        repository::UserRepository,
    },
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SignInRequestBody {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct SignInResponseBody {
    pub auth_token: String,
    pub refresh_token: String,
}

impl ApiResponder for SignInResponseBody {
    fn unit() -> &'static str {
        "sign in response payload"
    }
    fn article() -> &'static str {
        "A"
    }
}

#[derive(Debug, Serialize)]
pub struct InvalidationResponseBody {
    pub reason: InvalidationReason,
}

impl ApiResponder for InvalidationResponseBody {
    fn unit() -> &'static str {
        "invalidation response payload"
    }
    fn article() -> &'static str {
        "An"
    }
}

pub struct AuthHandlers<A: AuthRepository, U: UserRepository> {
    auth_repo: A,
    user_repo: U,
}

impl<A: AuthRepository, U: UserRepository> AuthHandlers<A, U> {
    pub fn new(auth_repo: A, user_repo: U) -> Self {
        Self {
            auth_repo,
            user_repo,
        }
    }

    pub async fn handle_signin(
        &self,
        body: SignInRequestBody,
    ) -> Result<DataResponse<SignInResponseBody>, ApiError> {
        let user = self
            .user_repo
            .get_by_email(body.email)
            .await?
            .ok_or(ApiError::AuthFailed)?;

        let auth_token = self
            .auth_repo
            .login_user(
                user.id,
                user.username,
                user.email,
                user.password,
                body.password,
            )
            .await?;

        let refresh_token = self.auth_repo.get_refresh_token(user.id).await?;

        Ok(SignInResponseBody {
            auth_token,
            refresh_token,
        }
        .into())
    }

    pub async fn handle_signup(
        &self,
        body: UserCreateData,
    ) -> Result<DataResponse<User>, ApiError> {
        let user = self.user_repo.create(UserRole::Common, body).await?;

        Ok(user.into())
    }

    pub async fn handle_get_self(
        &self,
        auth: UserAuthPayload,
    ) -> Result<DataResponse<User>, ApiError> {
        let user = self
            .user_repo
            .get_by_id(auth.sub)
            .await?
            .ok_or(ApiError::UserNotFound)?;

        Ok(user.into())
    }

    pub async fn handle_invalidate(
        &self,
        auth: UserAuthPayload,
    ) -> Result<DataResponse<InvalidationResponseBody>, ApiError> {
        const DEFAULT_REASON: InvalidationReason = InvalidationReason::Requested;

        self.auth_repo
            .add_invalidation(auth.sub, DEFAULT_REASON)
            .await?;

        Ok(InvalidationResponseBody {
            reason: DEFAULT_REASON,
        }
        .into())
    }
}
