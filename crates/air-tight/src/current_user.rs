use std::ops::Deref;

use air_tight_core::Principal;
use axum::extract::FromRequestParts;
use axum::http::request::Parts;

use crate::Error;

/// Extracts the authenticated principal that the app's own auth layer put
/// into the request extensions (for example with `Extension(user)` or
/// `request.extensions_mut().insert(user)`).
///
/// Rejects with [`Error::Unauthenticated`] (401) when no principal of type
/// `U` is present. `#[air_tight::permission]` injects this extractor into
/// every protected handler, which is why a missing principal answers 401
/// rather than axum's 500 for a missing `Extension`.
#[derive(Clone, Debug)]
pub struct CurrentUser<U>(pub U);

impl<U> Deref for CurrentUser<U> {
    type Target = U;

    fn deref(&self) -> &U {
        &self.0
    }
}

impl<U: Principal, S: Send + Sync> FromRequestParts<S> for CurrentUser<U> {
    type Rejection = Error;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Error> {
        parts
            .extensions
            .get::<U>()
            .cloned()
            .map(CurrentUser)
            .ok_or(Error::Unauthenticated)
    }
}
