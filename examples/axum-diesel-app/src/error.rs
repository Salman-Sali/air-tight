//! The example's error type and its HTTP mapping.
use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use diesel::result::{DatabaseErrorKind, Error as DieselError};
use serde_json::json;

/// Every way a request of the example can fail.
#[derive(Debug)]
pub enum ApiError {
    /// An authentication or authorization failure from air-tight (401 or 403).
    Auth(air_tight::Error),
    /// The addressed row does not exist (404).
    NotFound(&'static str),
    /// The request conflicts with existing data (409).
    Conflict(&'static str),
    /// The request is well-formed JSON but not acceptable (400).
    BadRequest(String),
    /// Something went wrong on our side (500).
    Internal,
}

impl From<air_tight::Error> for ApiError {
    fn from(err: air_tight::Error) -> Self {
        match err {
            air_tight::Error::Catalog(why) => ApiError::BadRequest(why),
            air_tight::Error::Db(err) => ApiError::from(err),
            other => ApiError::Auth(other),
        }
    }
}

impl From<DieselError> for ApiError {
    fn from(err: DieselError) -> Self {
        match err {
            DieselError::NotFound => ApiError::NotFound("not found"),
            DieselError::DatabaseError(DatabaseErrorKind::UniqueViolation, _) => {
                ApiError::Conflict("already exists")
            }
            DieselError::DatabaseError(DatabaseErrorKind::ForeignKeyViolation, _) => {
                ApiError::NotFound("a referenced row does not exist")
            }
            other => {
                eprintln!("database error: {other}");
                ApiError::Internal
            }
        }
    }
}

impl From<diesel::r2d2::PoolError> for ApiError {
    fn from(err: diesel::r2d2::PoolError) -> Self {
        eprintln!("pool error: {err}");
        ApiError::Internal
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            ApiError::Auth(err) => return err.into_response(),
            ApiError::NotFound(what) => (StatusCode::NOT_FOUND, what.to_owned()),
            ApiError::Conflict(what) => (StatusCode::CONFLICT, what.to_owned()),
            ApiError::BadRequest(why) => (StatusCode::BAD_REQUEST, why),
            ApiError::Internal => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "internal error".to_owned(),
            ),
        };
        (status, Json(json!({ "error": message }))).into_response()
    }
}
