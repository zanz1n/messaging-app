use crate::{
    errors::{ApiError, ErrorResponse},
    ENCODING_FAILED_BODY,
};
use async_trait::async_trait;
use axum::{
    extract::{rejection::JsonRejection, FromRequest, FromRequestParts},
    http::{header, request::Parts, HeaderValue, Request, StatusCode},
    response::IntoResponse,
    Extension,
};
use serde::Serialize;
use std::{any::type_name, sync::Arc};

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

impl ApiResponder for () {
    #[inline]
    fn unit() -> &'static str {
        "reponse with nothing"
    }
    #[inline]
    fn article() -> &'static str {
        "A"
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

#[derive(Debug, Clone, Default)]
pub struct AppData<T>(pub Arc<T>);

impl<T> AppData<T> {
    #[inline]
    pub fn new(data: Arc<T>) -> Self {
        Self(data)
    }

    #[inline]
    pub fn extension(data: T) -> Extension<Arc<T>> {
        Extension(Arc::new(data))
    }

    #[inline]
    pub fn extension_arc(data: Arc<T>) -> Extension<Arc<T>> {
        Extension(data)
    }
}

#[async_trait]
impl<T: Sync + Send + 'static, S> FromRequestParts<S> for AppData<T> {
    type Rejection = ErrorResponse;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let data = parts.extensions.get::<Arc<T>>().ok_or_else(|| {
            let t_name = type_name::<T>();
            let self_t_name = type_name::<Self>();

            tracing::error!(type_name = t_name, "Failed get AppData request extension");

            ApiError::ServicePanicked(Some(format!(
                "Failed to get '{self_t_name}' request extension"
            )))
        })?;

        Ok(Self::new(data.clone()))
    }
}

#[derive(Debug, Serialize)]
pub struct DataResponse<T: Serialize> {
    pub data: T,
    pub message: Option<String>,
    #[serde(skip_serializing)]
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
    #[inline]
    fn from(value: T) -> Self {
        Self {
            message: Some(value.message()),
            http_code: Some(value.http_code()),
            data: value,
        }
    }
}

pub struct Json<T>(pub T);

#[async_trait]
impl<S, B, T> FromRequest<S, B> for Json<T>
where
    axum::Json<T>: FromRequest<S, B, Rejection = JsonRejection>,
    S: Send + Sync,
    B: Send + 'static,
{
    type Rejection = ErrorResponse;

    async fn from_request(req: Request<B>, state: &S) -> Result<Self, Self::Rejection> {
        match axum::Json::from_request(req, state).await {
            Ok(axum::Json(v)) => Ok(Self(v)),
            Err(e) => {
                let status_code = e.status();
                Err(ErrorResponse {
                    error_code: u32::from(status_code.as_u16()) * 100_u32,
                    status_code,
                    message: e.body_text(),
                })
            }
        }
    }
}

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
