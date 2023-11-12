use crate::ENCODING_FAILED_BODY;
use axum::{
    body::BoxBody,
    http::{header, HeaderValue, Response, StatusCode},
    response::IntoResponse,
};
use serde::{Serialize, Serializer};

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ApiError {
    #[error("Server service panicked: {0:?}")]
    ServicePanicked(Option<String>),

    #[cfg(feature = "sqlx")]
    #[error("Something went wrong while fetching the data")]
    SqlxError,

    #[error("Websocket packets must be sent every {0} seconds")]
    /// The amount of seconds between a packet acknowledgement
    GatewayTimeout(u64),
    #[error("The received message does not contain valid utf8 characters")]
    GatewayMessageNonUTF8,
    #[error("The received message could not be deserialized: {0}")]
    /// The serde deserialization error string
    GatewayDeserializationFailed(String),

    #[error("Something went wrong")]
    CacheGetFailed,
    #[error("Something went wrong")]
    CacheSetFailed,
    #[error("Something went wrong")]
    CacheDeserializationFailed,
    #[error("Something went wrong")]
    CacheSerializationFailed,

    #[error("Something went wrong")]
    MessagingDeserializationFailed,
    #[error("Something went wrong")]
    MessagingSerializationFailed,
    #[error("Something went wrong")]
    MessagingSendError,
    #[error("Something went wrong")]
    MessagingRecvError,

    #[error("The message could not be found")]
    MessageNotFound,
    #[error("Failed to fetch the message")]
    MessageFetchFailed,
    #[error("You cannot edit a message you didn't send")]
    MessageEditDenied,
    #[error("You cannot delete a message if you don't own it or if you are not an admin")]
    MessageDeleteDenied,

    #[error("The user could not be found")]
    UserNotFound,
    #[error("Failed to fetch the user")]
    UserFetchFailed,
    #[error("The user already exists")]
    UserAlreadyExists,

    #[error("Authorization is required but the 'Authorization' header was not provided")]
    AuthHeaderMissing,
    #[error("Authorization is required but the 'Authorization' header is invalid")]
    AuthHeaderInvalid,
    #[error("User not found or password do now match")]
    AuthFailed,
    #[error("The provided auth token is invalid")]
    AuthTokenInvalid,
    #[error("The provided auth token is expired")]
    AuthTokenExpired,
    #[error("The provided refresh token is invalid")]
    AuthRefreshTokenInvalid,
    #[error("Failed to generate the authentication token")]
    AuthTokenGenerationFailed,
    #[error("Something went wrong")]
    AuthBcryptHashFailed,
    #[error("The user is under invalidation, please login again later")]
    AuthUserInvalidated,

    #[error("The channel could not be found")]
    ChannelNotFound,
    #[error("Failed to fetch the channel")]
    ChannelFetchFailed,
    #[error("You don't have permission to do this action in the channel")]
    ChannelPermissionDenied,
}

impl Serialize for ApiError {
    #[inline]
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        Into::<ErrorResponse>::into(self).serialize(serializer)
    }
}

impl Into<StatusCode> for &ApiError {
    #[inline]
    fn into(self) -> StatusCode {
        match self {
            #[cfg(feature = "sqlx")]
            ApiError::SqlxError => StatusCode::INTERNAL_SERVER_ERROR,
            ApiError::ServicePanicked(_)
            | ApiError::MessageFetchFailed
            | ApiError::AuthTokenGenerationFailed
            | ApiError::UserFetchFailed
            | ApiError::CacheGetFailed
            | ApiError::CacheSetFailed
            | ApiError::CacheDeserializationFailed
            | ApiError::CacheSerializationFailed
            | ApiError::MessagingDeserializationFailed
            | ApiError::MessagingSerializationFailed
            | ApiError::MessagingSendError
            | ApiError::MessagingRecvError
            | ApiError::AuthBcryptHashFailed
            | ApiError::ChannelFetchFailed => StatusCode::INTERNAL_SERVER_ERROR,
            ApiError::GatewayTimeout(_) => StatusCode::REQUEST_TIMEOUT,
            ApiError::GatewayDeserializationFailed(_) | ApiError::GatewayMessageNonUTF8 => {
                StatusCode::BAD_REQUEST
            }
            ApiError::UserAlreadyExists => StatusCode::CONFLICT,
            ApiError::AuthHeaderMissing
            | ApiError::AuthHeaderInvalid
            | ApiError::AuthFailed
            | ApiError::AuthTokenInvalid
            | ApiError::UserNotFound
            | ApiError::AuthTokenExpired
            | ApiError::AuthRefreshTokenInvalid
            | ApiError::AuthUserInvalidated
            | ApiError::ChannelNotFound => StatusCode::UNAUTHORIZED,
            ApiError::MessageNotFound => StatusCode::NOT_FOUND,
            ApiError::MessageEditDenied
            | ApiError::MessageDeleteDenied
            | ApiError::ChannelPermissionDenied => StatusCode::FORBIDDEN,
        }
    }
}

impl Into<u32> for &ApiError {
    #[inline]
    fn into(self) -> u32 {
        match self {
            #[cfg(feature = "sqlx")]
            ApiError::SqlxError => 50000,
            ApiError::CacheGetFailed
            | ApiError::CacheSetFailed
            | ApiError::CacheDeserializationFailed
            | ApiError::CacheSerializationFailed
            | ApiError::MessagingDeserializationFailed
            | ApiError::MessagingSerializationFailed
            | ApiError::MessagingSendError
            | ApiError::MessagingRecvError
            | ApiError::AuthBcryptHashFailed => 50000,
            ApiError::ServicePanicked(_) => 50001,
            ApiError::GatewayTimeout(_) => 40801,
            ApiError::GatewayMessageNonUTF8 => 40001,
            ApiError::GatewayDeserializationFailed(_) => 40002,
            ApiError::MessageNotFound => 40401,
            ApiError::MessageFetchFailed => 50002,
            ApiError::MessageEditDenied => 40301,
            ApiError::MessageDeleteDenied => 40302,
            ApiError::UserNotFound => 40402,
            ApiError::UserFetchFailed => 50003,
            ApiError::UserAlreadyExists => 40901,
            ApiError::AuthHeaderMissing => 40101,
            ApiError::AuthHeaderInvalid => 40102,
            ApiError::AuthFailed => 40103,
            ApiError::AuthTokenInvalid => 40104,
            ApiError::AuthTokenExpired => 40105,
            ApiError::AuthRefreshTokenInvalid => 40106,
            ApiError::AuthUserInvalidated => 40107,
            ApiError::AuthTokenGenerationFailed => 50004,
            ApiError::ChannelNotFound => 40403,
            ApiError::ChannelFetchFailed => 50005,
            ApiError::ChannelPermissionDenied => 40303,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub message: String,
    pub error_code: u32,
    #[serde(skip_serializing)]
    pub status_code: StatusCode,
}

impl ErrorResponse {
    #[inline]
    pub fn new(message: String, error_code: u32, status_code: StatusCode) -> Self {
        Self {
            message,
            error_code,
            status_code,
        }
    }
}

impl IntoResponse for ErrorResponse {
    fn into_response(self) -> axum::response::Response {
        let tuple = match serde_json::to_vec(&self) {
            Ok(buf) => (
                self.status_code,
                [(
                    header::CONTENT_TYPE,
                    HeaderValue::from_static(mime::APPLICATION_JSON.as_ref()),
                )],
                buf,
            ),
            Err(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                [(
                    header::CONTENT_TYPE,
                    HeaderValue::from_static(mime::APPLICATION_JSON.as_ref()),
                )],
                ENCODING_FAILED_BODY.to_vec(),
            ),
        };

        tuple.into_response()
    }
}

impl From<&ApiError> for ErrorResponse {
    #[inline]
    fn from(value: &ApiError) -> Self {
        ErrorResponse {
            error_code: value.into(),
            status_code: value.into(),
            message: value.to_string(),
        }
    }
}

impl From<ApiError> for ErrorResponse {
    #[inline]
    fn from(value: ApiError) -> Self {
        (&value).into()
    }
}

impl IntoResponse for ApiError {
    #[inline]
    fn into_response(self) -> Response<BoxBody> {
        ErrorResponse::new(self.to_string(), (&self).into(), (&self).into()).into_response()
    }
}
