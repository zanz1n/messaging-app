use crate::http::ApiResponder;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE", deny_unknown_fields)]
pub enum UserRole {
    Admin,
    Common,
}

impl UserRole {
    #[inline]
    pub(super) fn to_upper_enum(&self) -> &'static str {
        match self {
            UserRole::Admin => "ADMIN",
            UserRole::Common => "COMMON",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct User {
    pub id: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub email: String,
    pub username: String,
    pub role: UserRole,
    #[serde(skip_serializing)]
    pub password: String,
}

impl ApiResponder for User {
    fn unit() -> &'static str {
        "user"
    }
    fn article() -> &'static str {
        "An"
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UserCreateData {
    pub email: String,
    pub username: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UserUpdateData {
    pub username: Option<String>,
}

#[derive(Debug, Clone)]
pub(super) enum UserUpdateVariant {
    Username(String),
    None,
}

impl Into<UserUpdateVariant> for UserUpdateData {
    fn into(self) -> UserUpdateVariant {
        if let Some(username) = self.username {
            UserUpdateVariant::Username(username)
        } else {
            UserUpdateVariant::None
        }
    }
}

#[cfg(feature = "sqlx")]
mod sqlx {
    use super::{User, UserRole};
    use chrono::{DateTime, Utc};
    use sqlx::{
        database::{HasArguments, HasValueRef},
        encode::IsNull,
        error::BoxDynError,
        ColumnIndex, Database, Decode, Encode, FromRow, Row, Type,
    };
    use uuid::Uuid;

    impl<'e, DB: Database> Encode<'e, DB> for UserRole
    where
        &'e str: Encode<'e, DB>,
    {
        #[inline]
        fn encode_by_ref(&self, buf: &mut <DB as HasArguments<'e>>::ArgumentBuffer) -> IsNull {
            Encode::<'e, DB>::encode(self, buf)
        }

        #[inline]
        fn size_hint(&self) -> usize {
            Encode::<'e, DB>::size_hint(&self.to_upper_enum())
        }
    }

    impl<'de, R: Row> FromRow<'de, R> for User
    where
        &'de str: ColumnIndex<R>,
        Uuid: Decode<'de, R::Database> + Type<R::Database>,
        DateTime<Utc>: Decode<'de, R::Database> + Type<R::Database>,
        String: Decode<'de, R::Database> + Type<R::Database>,
        UserRole: Decode<'de, R::Database> + Type<R::Database>,
    {
        fn from_row(row: &'de R) -> Result<Self, sqlx::Error> {
            let user = Self {
                id: row.try_get("id")?,
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
                email: row.try_get("email")?,
                username: row.try_get("username")?,
                role: row.try_get("role")?,
                password: row.try_get("password")?,
            };

            Ok(user)
        }
    }

    impl<'de, DB: Database> Decode<'de, DB> for UserRole
    where
        &'de str: Decode<'de, DB>,
    {
        fn decode(value: <DB as HasValueRef<'de>>::ValueRef) -> Result<Self, BoxDynError> {
            let value = Decode::<'de, DB>::decode(value)?;
            match value {
                "ADMIN" => ::std::result::Result::Ok(UserRole::Admin),
                "COMMON" => ::std::result::Result::Ok(UserRole::Common),
                _ => Err(format!("invalid value {value:?} for enum UserRole").into()),
            }
        }
    }
}
