use crate::errors::ApiError;
use axum::{body::BoxBody, http::Response, response::IntoResponse};
use std::{
    env,
    fmt::{Debug, Display},
    str::FromStr,
};
use tower_http::catch_panic::ResponseForPanic;

#[derive(Debug, Default, Clone, Copy)]
pub struct JsonPanicHandler;

impl ResponseForPanic for JsonPanicHandler {
    type ResponseBody = BoxBody;

    fn response_for_panic(
        &mut self,
        err: Box<dyn std::any::Any + Send + 'static>,
    ) -> Response<Self::ResponseBody> {
        if let Some(s) = err.downcast_ref::<String>() {
            tracing::error!("Service panicked: {}", s);

            ApiError::ServicePanicked(Some(s))
        } else if let Some(s) = err.downcast_ref::<&str>() {
            tracing::error!("Service panicked: {}", s);

            ApiError::ServicePanicked(Some(*s))
        } else {
            tracing::error!(
                "Service panicked but `CatchPanic` was unable to downcast the panic info"
            );

            ApiError::ServicePanicked(None)
        }
        .into_response()
    }
}

#[cfg(feature = "http_cors")]
use axum::routing::Router;

#[cfg(feature = "http_cors")]
pub fn setup_app_cors(app: Router) -> Router {
    use std::time::Duration;
    use tower_http::cors::{
        AllowHeaders, AllowMethods, AllowOrigin, AllowPrivateNetwork, CorsLayer, ExposeHeaders,
        MaxAge,
    };

    let max_age = env_param("APP_CORS_MAX_AGE").unwrap_or(3600_u64);

    app.layer(
        CorsLayer::new()
            .allow_headers(AllowHeaders::any())
            .allow_methods(AllowMethods::any())
            .allow_origin(AllowOrigin::any())
            .allow_private_network(AllowPrivateNetwork::yes())
            .expose_headers(ExposeHeaders::any())
            .max_age(MaxAge::exact(Duration::from_secs(max_age))),
    )
}

#[derive(thiserror::Error)]
pub enum VarError {
    #[cfg(feature = "dotenv")]
    #[error("The dotenv file could not be found")]
    DotenvFileNotFound,

    #[error("The environment variable \"{0}\" was not provided")]
    NotProvided(&'static str),
    #[error("The environment variable \"{0}\" could not be parsed")]
    Invalid(&'static str),
}

impl Debug for VarError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self, f)
    }
}

pub fn env_param<T: FromStr>(key: &'static str) -> Result<T, VarError> {
    impl VarError {
        fn from_std(err: env::VarError, key: &'static str) -> Self {
            match err {
                env::VarError::NotPresent => Self::NotProvided(key),
                env::VarError::NotUnicode(_) => Self::Invalid(key),
            }
        }
    }

    match env::var(key) {
        Ok(v) => T::from_str(&v).map_err(|_| VarError::Invalid(key)),
        Err(err) => Err(VarError::from_std(err, key)),
    }
}
