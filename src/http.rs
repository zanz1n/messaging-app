use crate::ENCODING_FAILED_BODY;
use axum::{
    http::{header, HeaderValue, StatusCode},
    response::IntoResponse,
};
use serde::Serialize;

pub fn marshal_json_string<T: Serialize>(value: &T) -> String {
    match serde_json::to_string(value) {
        Ok(v) => v,
        Err(e) => {
            tracing::error!(error = e.to_string(), "Failed to encode json");

            unsafe { String::from_utf8_unchecked(ENCODING_FAILED_BODY.to_vec()) }
        }
    }
}

pub fn marshal_json_vec<T: Serialize, R: From<Vec<u8>>>(value: &T) -> R {
    match serde_json::to_vec(value) {
        Ok(v) => R::from(v),
        Err(e) => {
            tracing::error!(error = e.to_string(), "Failed to encode json");

            R::from(ENCODING_FAILED_BODY.to_vec())
        }
    }
}

pub trait ApiResponder {
    fn http_code(&self) -> StatusCode {
        StatusCode::OK
    }

    fn unit() -> &'static str;
    fn article() -> &'static str;

    fn message(&self) -> String {
        format!("{} {} was returned", Self::article(), Self::unit())
    }
}

impl<T: ApiResponder + Serialize> ApiResponder for Vec<T> {
    #[inline]
    fn unit() -> &'static str {
        T::unit()
    }

    #[inline]
    fn article() -> &'static str {
        T::article()
    }

    fn message(&self) -> String {
        let unit = Self::unit();

        match self.len() {
            0 => format!("No {unit} was returned"),
            1 => format!("1 {unit} was returned"),
            n => format!("{n} {unit} were returned"),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct DataResponse<T: Serialize> {
    pub data: T,
    pub message: Option<String>,
    #[serde(skip)]
    pub http_code: Option<StatusCode>,
}

impl<T: ApiResponder + Serialize> IntoResponse for DataResponse<T> {
    fn into_response(mut self) -> axum::response::Response {
        if self.http_code.is_none() {
            self.http_code = Some(self.data.http_code());
        }
        if self.message.is_none() {
            self.message = Some(self.data.message());
        }

        let tuple = match serde_json::to_vec(&self) {
            Ok(buf) => (
                self.http_code.unwrap(),
                [(
                    header::CONTENT_TYPE,
                    HeaderValue::from_static(mime::APPLICATION_JSON.as_ref()),
                )],
                buf,
            ),
            Err(e) => {
                tracing::error!({ error = e.to_string() }, "Failed to encode response body");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    [(
                        header::CONTENT_TYPE,
                        HeaderValue::from_static(mime::APPLICATION_JSON.as_ref()),
                    )],
                    ENCODING_FAILED_BODY.to_vec(),
                )
            }
        };

        tuple.into_response()
    }
}

impl<T: ApiResponder + Serialize> From<T> for DataResponse<T> {
    fn from(value: T) -> Self {
        Self {
            message: Some(value.message()),
            http_code: Some(value.http_code()),
            data: value,
        }
    }
}
