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
    WebsocketTimeout(u64),
    #[error("The received message does not contain valid utf8 characters")]
    WebsocketMessageNonUTF8,
    #[error("The received message could not be deserialized: {0}")]
    WebsocketMessageDeserializationFailed(String),
    #[error("Something went wrong: {0}")]
    CustomServerError(&'a str),
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
            ApiError::CustomServerError(_) | ApiError::ServicePanicked(_) => {
                StatusCode::INTERNAL_SERVER_ERROR
            }
            ApiError::WebsocketTimeout(_) => StatusCode::REQUEST_TIMEOUT,
            ApiError::WebsocketMessageDeserializationFailed(_)
            | ApiError::WebsocketMessageNonUTF8 => StatusCode::BAD_REQUEST,
        }
    }
}

impl<'a> Into<u32> for &ApiError<'a> {
    fn into(self) -> u32 {
        match self {
            ApiError::CustomServerError(_) => 50000,
            ApiError::ServicePanicked(_) => 50001,
            ApiError::WebsocketTimeout(_) => 40801,
            ApiError::WebsocketMessageNonUTF8 => 40001,
            ApiError::WebsocketMessageDeserializationFailed(_) => 40002,
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
