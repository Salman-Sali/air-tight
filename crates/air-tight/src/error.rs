use air_tight_core::PermissionKey;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

/// The single error type of air-tight.
///
/// Lives in the runtime crate (not core) because of the diesel variant.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// No authenticated principal was present.
    #[error("unauthenticated")]
    Unauthenticated,
    /// The principal lacks the required permissions.
    #[error("forbidden, missing: {missing:?}")]
    Forbidden {
        /// Keys of the permissions that were missing.
        missing: Vec<String>,
    },
    /// A database error.
    #[error("database error: {0}")]
    Db(#[from] diesel::result::Error),
    /// A connection pool error.
    #[error("connection pool error")]
    Pool,
    /// A catalog synchronisation or validation error.
    #[error("catalog error: {0}")]
    Catalog(String),
    /// A LISTEN/NOTIFY error.
    #[error("notify error: {0}")]
    Notify(String),
}

impl Error {
    /// Builds a `Forbidden` error from missing permission keys.
    pub fn forbidden(missing: Vec<PermissionKey>) -> Self {
        Error::Forbidden {
            missing: missing.into_iter().map(|k| k.as_str().to_owned()).collect(),
        }
    }
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        let status = match self {
            Error::Unauthenticated => StatusCode::UNAUTHORIZED,
            Error::Forbidden { .. } => StatusCode::FORBIDDEN,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };
        (status, status.canonical_reason().unwrap_or("error")).into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_codes() {
        assert_eq!(Error::Unauthenticated.into_response().status(), 401);
        assert_eq!(Error::forbidden(vec![]).into_response().status(), 403);
        assert_eq!(Error::Pool.into_response().status(), 500);
        assert_eq!(Error::Catalog("x".into()).into_response().status(), 500);
    }
}
