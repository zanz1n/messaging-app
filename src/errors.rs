use crate::ENCODING_FAILED_BODY;
use axum::{
    body::BoxBody,
    http::{header, HeaderValue, Response, StatusCode},
    response::IntoResponse,
};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct ErrorBody {
    pub message: String,
    pub error_code: u32,
}

impl ErrorBody {
    #[inline]
    pub fn new(message: String, error_code: u32) -> Self {
        Self {
            message,
            error_code,
        }
    }
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum ApiError<'a> {
    #[error("Server service panicked: {0:?}")]
    ServicePanicked(Option<&'a str>),

    #[error("Websocket packets must be sent every {0} seconds")]
    /// The amount of seconds between a packet acknowledgement
    GatewayTimeout(u64),
    #[error("The received message does not contain valid utf8 characters")]
    GatewayMessageNonUTF8,
    #[error("The received message could not be deserialized: {0}")]
    /// The serde deserialization error string
    GatewayDeserializationFailed(String),

    #[error("The message could not be found")]
    MessageNotFound,
    #[error("Failed to fetch the message")]
    MessageFetchFailed,
}

impl<'a> Serialize for ApiError<'a> {
    #[inline]
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        ErrorBody {
            error_code: self.into(),
            message: self.to_string(),
        }
        .serialize(serializer)
    }
}

impl<'a> Into<StatusCode> for &ApiError<'a> {
    fn into(self) -> StatusCode {
        match self {
            ApiError::ServicePanicked(_) | ApiError::MessageFetchFailed => {
                StatusCode::INTERNAL_SERVER_ERROR
            }
            ApiError::GatewayTimeout(_) => StatusCode::REQUEST_TIMEOUT,
            ApiError::GatewayDeserializationFailed(_) | ApiError::GatewayMessageNonUTF8 => {
                StatusCode::BAD_REQUEST
            }
            ApiError::MessageNotFound => StatusCode::NOT_FOUND,
        }
    }
}

impl<'a> Into<u32> for &ApiError<'a> {
    fn into(self) -> u32 {
        match self {
            ApiError::ServicePanicked(_) => 50001,
            ApiError::GatewayTimeout(_) => 40801,
            ApiError::GatewayMessageNonUTF8 => 40001,
            ApiError::GatewayDeserializationFailed(_) => 40002,
            ApiError::MessageNotFound => 40401,
            ApiError::MessageFetchFailed => 50002,
        }
    }
}

impl<'a> IntoResponse for ApiError<'a> {
    fn into_response(self) -> Response<BoxBody> {
        let err_body = ErrorBody::new(self.to_string(), (&self).into());

        let tuple = match serde_json::to_vec(&err_body) {
            Ok(buf) => (
                (&self).into(),
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
