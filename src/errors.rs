use axum::{
    body::BoxBody,
    http::{header, HeaderValue, Response, StatusCode},
    response::IntoResponse,
};
use serde::Serialize;

use crate::ENCODING_FAILED_BODY;

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

#[derive(Debug, thiserror::Error)]
pub enum ApiError<'a> {
    #[error("Server service panicked: {0:?}")]
    ServicePanicked(Option<&'a str>),
    #[error("Something went wrong: {0}")]
    CustomServerError(&'a str),
}

impl<'a> Into<StatusCode> for &ApiError<'a> {
    fn into(self) -> StatusCode {
        match self {
            ApiError::CustomServerError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            ApiError::ServicePanicked(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl<'a> Into<u32> for &ApiError<'a> {
    fn into(self) -> u32 {
        match self {
            ApiError::CustomServerError(_) => 50000,
            ApiError::ServicePanicked(_) => 50001,
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
