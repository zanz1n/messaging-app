use super::{models::UserAuthPayload, repository::AuthRepository};
use crate::errors::{ApiError, ErrorResponse};
use async_trait::async_trait;
use axum::{
    extract::FromRequestParts,
    http::{header, request::Parts},
};
use std::{any::type_name, marker::PhantomData};

pub struct AuthExtractor<T: AuthRepository>(pub UserAuthPayload, pub PhantomData<T>);

#[async_trait]
impl<T: AuthRepository + 'static, S: Send + Sync> FromRequestParts<S> for AuthExtractor<T> {
    type Rejection = ErrorResponse;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let auth_header = match parts.headers.get_mut(header::AUTHORIZATION) {
            Some(v) => {
                v.set_sensitive(true);
                v.to_str().or(Err(ApiError::AuthHeaderInvalid))?
            }
            None => return Err(ApiError::AuthHeaderMissing.into()),
        };

        if !auth_header.starts_with("Bearer ") || 10 > auth_header.len() {
            return Err(ApiError::AuthHeaderInvalid.into());
        }
        let (_, token) = auth_header.split_at(7);

        let repo = parts.extensions.get::<T>().ok_or_else(|| {
            let t_name = type_name::<T>();

            tracing::error!(
                type_name = t_name,
                "Failed to get AuthRepository impl request extension"
            );

            ApiError::ServicePanicked(Some(format!("Failed to get '{t_name}' request extension")))
        })?;

        let payload = repo.auth_user(token.to_string()).await?;

        let invalidation = repo.is_invalidated(payload.sub).await?;
        if let Some(invalidation) = invalidation {
            if (invalidation.created_at.timestamp() as u64) + 10 > payload.iat {
                return Err(ApiError::AuthUserInvalidated.into());
            }
        }

        Ok(Self(payload, PhantomData))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        auth::{
            jwt_repository::JwtAuthRepository, models::InvalidationReason,
            repository::AuthRepository,
        },
        cache::memory_repository::InMemoryCacheRepository,
    };
    use axum::{
        body::Body,
        http::{Method, Request},
    };
    use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey};
    use std::time::Duration;
    use uuid::Uuid;

    type InMemoryAuthRepository = JwtAuthRepository<InMemoryCacheRepository>;

    async fn mock_must_success_req(
        ar: InMemoryAuthRepository,
        token: &str,
        uuid: Uuid,
        email: &str,
        username: &str,
    ) {
        let req = Request::builder()
            .extension(ar)
            .method(Method::POST)
            .uri("/")
            .header(header::AUTHORIZATION, format!("Bearer {token}"))
            .body(Body::empty())
            .unwrap();

        let (mut parts, b) = req.into_parts();
        drop(b);

        let AuthExtractor(ap, _) =
            AuthExtractor::<InMemoryAuthRepository>::from_request_parts(&mut parts, &())
                .await
                .unwrap();

        assert_eq!(ap.sub, uuid);
        assert_eq!(ap.email, email);
        assert_eq!(ap.username, username);
    }

    async fn mock_must_fail_req(ar: InMemoryAuthRepository, token: &str) {
        let req = Request::builder()
            .extension(ar.clone())
            .method(Method::POST)
            .uri("/")
            .header(header::AUTHORIZATION, format!("Bearer {token}"))
            .body(Body::empty())
            .unwrap();

        let (mut parts, b) = req.into_parts();
        drop(b);

        AuthExtractor::<InMemoryAuthRepository>::from_request_parts(&mut parts, &())
            .await
            .err()
            .unwrap();
    }

    #[tokio::test]
    async fn test_auth_extractor() {
        const RANDOM_BASE64_STRING: &'static str =
            "YYX3sUuIw9wbAQOL3XOUkOwWE5JCx32VLae5t0mo7Zpqx17PT9UFl58Yj3QQetBn";

        let uuid = Uuid::new_v4();
        let username = "izanrodrigues";
        let email = "izanrodrigues999@gmail.com";

        let ar = JwtAuthRepository::new(
            Algorithm::HS512,
            EncodingKey::from_base64_secret(RANDOM_BASE64_STRING).unwrap(),
            DecodingKey::from_base64_secret(RANDOM_BASE64_STRING).unwrap(),
            3,
            InMemoryCacheRepository::new(),
        );

        let token = ar
            .generate_token(uuid, username.into(), email.into())
            .await
            .unwrap();

        mock_must_success_req(ar.clone(), &token, uuid, email, username).await;

        ar.add_invalidation(uuid, InvalidationReason::Requested)
            .await
            .unwrap();

        mock_must_fail_req(ar.clone(), &token).await;

        tokio::time::sleep(Duration::from_secs(15)).await;

        mock_must_success_req(ar.clone(), &token, uuid, email, username).await;
    }
}
