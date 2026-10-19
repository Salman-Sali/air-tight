use std::fmt;
use std::marker::PhantomData;
use std::ops::Deref;

use air_tight_core::{IdentifyResources, ResourceKinds, ResourceScope};
use axum::extract::rejection::QueryRejection;
use axum::extract::{FromRequestParts, Query};
use axum::http::request::Parts;
use serde::de::DeserializeOwned;

/// A query string extractor that also names the resources it carries.
///
/// A thin wrapper over [`axum::extract::Query`]. For a struct `T` that
/// derives `IdentifyResources`, use `K = ()` (`AirTightQuery<Filter, ()>`) and
/// the struct's own derive names the resources. The tuple forms of
/// [`ResourceKinds`] also apply, but a query string cannot be deserialized into
/// a bare tuple (`serde_urlencoded` only supports maps and structs), so in
/// practice use a struct.
///
/// `#[air_tight::permission]` includes parameters of this type in the
/// request's resource scopes automatically.
pub struct AirTightQuery<T, K>(pub T, PhantomData<fn() -> K>);

impl<T, K> AirTightQuery<T, K> {
    /// Unwraps the extracted value.
    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T, K> Deref for AirTightQuery<T, K> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.0
    }
}

impl<T: fmt::Debug, K> fmt::Debug for AirTightQuery<T, K> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("AirTightQuery").field(&self.0).finish()
    }
}

impl<T: Clone, K> Clone for AirTightQuery<T, K> {
    fn clone(&self) -> Self {
        AirTightQuery(self.0.clone(), PhantomData)
    }
}

impl<T, K: ResourceKinds<T>> IdentifyResources for AirTightQuery<T, K> {
    fn resource_scopes(&self) -> Vec<ResourceScope> {
        K::scopes(&self.0)
    }
}

impl<S, T, K> FromRequestParts<S> for AirTightQuery<T, K>
where
    S: Send + Sync,
    T: DeserializeOwned,
{
    type Rejection = QueryRejection;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, QueryRejection> {
        let Query(value) = Query::<T>::from_request_parts(parts, state).await?;
        Ok(AirTightQuery(value, PhantomData))
    }
}
