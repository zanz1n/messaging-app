use super::{
    models::{User, UserCreateData, UserRole, UserUpdateData, UserUpdateVariant},
    repository::UserRepository,
};
use crate::errors::ApiError;
use async_trait::async_trait;
use sqlx::{postgres::PgTypeInfo, Pool, Postgres, Type};
use tokio::task::spawn_blocking;
use uuid::Uuid;

impl Type<Postgres> for UserRole {
    fn type_info() -> PgTypeInfo {
        PgTypeInfo::with_name("userrole")
    }
}

pub struct PostgresUserRepository {
    pool: Pool<Postgres>,
    bcrypt_cost: u32,
}

impl PostgresUserRepository {
    pub fn new(pool: Pool<Postgres>, bcrypt_cost: u32) -> Self {
        Self { pool, bcrypt_cost }
    }
}

#[async_trait]
impl UserRepository for PostgresUserRepository {
    async fn get_by_id(&self, id: Uuid) -> Result<Option<User>, ApiError> {
        let res = sqlx::query_as(r#"SELECT * FROM "users" where "id" = $1"#)
            .bind(id)
            .fetch_one(&self.pool)
            .await;

        match res {
            Ok(v) => Ok(Some(v)),
            Err(e) => {
                if matches!(e, sqlx::Error::RowNotFound) {
                    Ok(None)
                } else {
                    tracing::error!(
                        error = e.to_string(),
                        method = "get_by_id",
                        "PostgresUserRepository sqlx error"
                    );

                    Err(ApiError::SqlxError)
                }
            }
        }
    }

    async fn get_by_email(&self, email: String) -> Result<Option<User>, ApiError> {
        let res = sqlx::query_as(r#"SELECT * FROM "users" where "email" = $1"#)
            .bind(email)
            .fetch_one(&self.pool)
            .await;

        match res {
            Ok(v) => Ok(Some(v)),
            Err(e) => {
                if matches!(e, sqlx::Error::RowNotFound) {
                    Ok(None)
                } else {
                    tracing::error!(
                        error = e.to_string(),
                        method = "get_by_email",
                        "PostgresUserRepository sqlx error"
                    );

                    Err(ApiError::SqlxError)
                }
            }
        }
    }

    async fn create(&self, role: UserRole, data: UserCreateData) -> Result<User, ApiError> {
        let id = Uuid::new_v4();

        let cost = self.bcrypt_cost;
        let passwd = spawn_blocking(move || {
            bcrypt::hash(data.password, cost).map_err(|e| {
                tracing::error!(
                    user_id = id.to_string(),
                    error = e.to_string(),
                    "Failed to hash password while creating user"
                );
                ApiError::AuthBcryptHashFailed
            })
        })
        .await
        .map_err(|e| {
            tracing::error!(error = e.to_string(), "Failed to spawn blocking");
            ApiError::AuthBcryptHashFailed
        })??;

        sqlx::query_as(
            r#"INSERT INTO "users"
            ("id", "email", "username", "role", "password")
            VALUES ($1, $2, $3, $4, $5)
            RETURNING *"#,
        )
        .bind(id)
        .bind(data.email)
        .bind(data.username)
        .bind(role)
        .bind(passwd)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| {
            if let sqlx::Error::Database(_) = e {
                ApiError::UserAlreadyExists
            } else {
                tracing::error!(
                    error = e.to_string(),
                    method = "create",
                    "PostgresUserRepository sqlx error"
                );

                ApiError::SqlxError
            }
        })
    }

    async fn update(&self, id: Uuid, data: UserUpdateData) -> Result<User, ApiError> {
        let query_as = match data.into() {
            UserUpdateVariant::None => {
                return self.get_by_id(id).await?.ok_or(ApiError::UserNotFound);
            }
            UserUpdateVariant::Username(u) => {
                sqlx::query_as(r#"UPDATE "users" SET "username" = $1 WHERE "id" = $1 RETURNING *"#)
                    .bind(u)
            }
        }
        .bind(id);

        query_as.fetch_one(&self.pool).await.map_err(|e| {
            if matches!(e, sqlx::Error::RowNotFound) {
                ApiError::UserNotFound
            } else {
                tracing::error!(
                    error = e.to_string(),
                    method = "update",
                    "PostgresUserRepository sqlx error"
                );

                ApiError::SqlxError
            }
        })
    }

    async fn delete(&self, id: Uuid) -> Result<(), ApiError> {
        let res = sqlx::query(r#"DELETE FROM "users" WHERE id = $1"#)
            .bind(id)
            .execute(&self.pool)
            .await;

        match res {
            Ok(r) => {
                if r.rows_affected() == 0 {
                    Err(ApiError::UserNotFound)
                } else {
                    Ok(())
                }
            }
            Err(e) => {
                if matches!(e, sqlx::Error::RowNotFound) {
                    Err(ApiError::UserNotFound)
                } else {
                    tracing::error!(
                        error = e.to_string(),
                        method = "delete",
                        "PostgresUserRepository sqlx error"
                    );

                    Err(ApiError::SqlxError)
                }
            }
        }
    }
}
