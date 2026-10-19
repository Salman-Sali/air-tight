use std::fmt;
use std::marker::PhantomData;
use std::ops::Deref;

use air_tight_core::{IdentifyResources, ResourceKinds, ResourceScope};
use axum::extract::rejection::PathRejection;
use axum::extract::{FromRequestParts, Path};
use axum::http::request::Parts;
use serde::de::DeserializeOwned;

/// A path extractor that also names the resources it carries.
///
/// A thin wrapper over [`axum::extract::Path`]: `T` is what the path
/// deserializes into and `K` lists the resource kinds of its parts. `K` is a
/// resource kind marker for a single id (`AirTightPath<Uuid, Workspace>`) or a
/// tuple of markers with the same arity as a tuple `T` (arities 1 to 8, for
/// example `AirTightPath<(Uuid, Uuid), (Workspace, Invoice)>`). See
/// [`ResourceKinds`] for the full rules.
///
/// `#[air_tight::permission]` includes parameters of this type in the
/// request's resource scopes automatically, so they need not be listed in
/// `resources = [..]`.
///
/// The field is public for destructuring (`AirTightPath(id, _)`); the second
/// field is a private marker, so prefer [`AirTightPath::into_inner`] or
/// `Deref`.
pub struct AirTightPath<T, K>(pub T, PhantomData<fn() -> K>);

impl<T, K> AirTightPath<T, K> {
    /// Unwraps the extracted value.
    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T, K> Deref for AirTightPath<T, K> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.0
    }
}

impl<T: fmt::Debug, K> fmt::Debug for AirTightPath<T, K> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("AirTightPath").field(&self.0).finish()
    }
}

impl<T: Clone, K> Clone for AirTightPath<T, K> {
    fn clone(&self) -> Self {
        AirTightPath(self.0.clone(), PhantomData)
    }
}

impl<T, K: ResourceKinds<T>> IdentifyResources for AirTightPath<T, K> {
    fn resource_scopes(&self) -> Vec<ResourceScope> {
        K::scopes(&self.0)
    }
}

impl<S, T, K> FromRequestParts<S> for AirTightPath<T, K>
where
    S: Send + Sync,
    T: DeserializeOwned + Send,
{
    type Rejection = PathRejection;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, PathRejection> {
        let Path(value) = Path::<T>::from_request_parts(parts, state).await?;
        Ok(AirTightPath(value, PhantomData))
    }
}
