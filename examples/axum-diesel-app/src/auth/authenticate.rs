use axum::extract::{Request, State};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use diesel::prelude::*;

use super::{AuthenticatedUser, UserId};
use crate::db::Db;
use crate::error::ApiError;
use crate::models::UserRecord;
use crate::schema::users;

/// Name of the header that carries the caller's user id.
pub const USER_HEADER: &str = "x-user-id";

/// Fake authentication middleware.
///
/// Reads `X-User-Id`, loads that user from `users` and inserts an
/// [`AuthenticatedUser`] into the request extensions. A missing, malformed or
/// unknown id answers 401.
pub async fn authenticate(State(db): State<Db>, mut req: Request, next: Next) -> Response {
    let Some(id) = req
        .headers()
        .get(USER_HEADER)
        .and_then(|value| value.to_str().ok())
        .and_then(UserId::parse)
    else {
        return unauthenticated();
    };
    let found = db
        .run(move |conn| {
            Ok(users::table
                .find(id.as_uuid())
                .select(UserRecord::as_select())
                .first(conn)
                .optional()?)
        })
        .await;
    match found {
        Ok(Some(row)) => {
            req.extensions_mut().insert(AuthenticatedUser::from(row));
            next.run(req).await
        }
        Ok(None) => unauthenticated(),
        Err(err) => err.into_response(),
    }
}

fn unauthenticated() -> Response {
    ApiError::Auth(air_tight::Error::Unauthenticated).into_response()
}
