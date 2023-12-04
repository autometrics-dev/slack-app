pub mod models;
pub mod sqlite;

pub use sqlite::Db;

use serde_with::SerializeDisplay;
use strum_macros::IntoStaticStr;
use thiserror::Error;

#[derive(Debug, Error, IntoStaticStr, SerializeDisplay)]
#[strum(serialize_all = "snake_case")]
pub enum DbError {
    #[error("entity not found")]
    NotFound,

    #[error("unknown database error")]
    UnknownError,

    #[error("database error: {0}")]
    DatabaseError(sqlx::Error),

    #[error("inconsistent state")]
    InconsistentState,

    #[error("duplicate row error")]
    DuplicateRow,

    #[error("invalid token")]
    InvalidToken,

    #[error("invalid arguments")]
    InvalidArguments,

    #[error("foreign key constraint violation")]
    ForeignKeyConstraintViolation,
}

impl From<sqlx::Error> for DbError {
    fn from(error: sqlx::Error) -> Self {
        use sqlx::Error::*;
        match error {
            RowNotFound => DbError::NotFound,
            err => DbError::DatabaseError(err),
        }
    }
}
