use std::convert::Infallible;

use axum::extract::Request;
use axum::response::IntoResponse;
use axum::routing::Route;
use tower::{Layer, Service};

use super::{MethodRouter, RouteRecord};

/// A wrapper over `axum::Router` whose `route` only accepts
/// [`MethodRouter`]s built from guarded or public handlers.
///
/// Every route is recorded so [`AirTight::audit_routes`](crate::AirTight::audit_routes)
/// can check it against the handler registry at startup.
///
/// Routers converted from a plain `axum::Router` (via `From`, then `merge`
/// or `nest`) bypass the typed gate: their routes are not recorded, so the
/// audit only covers them if they were built from this type.
pub struct Router<S = ()> {
    inner: axum::Router<S>,
    routes: Vec<RouteRecord>,
}

impl<S> Router<S> {
    /// Every recorded route.
    pub fn routes(&self) -> &[RouteRecord] {
        &self.routes
    }
}

impl<S: Clone + Send + Sync + 'static> Default for Router<S> {
    fn default() -> Self {
        Self::new()
    }
}

impl<S: Clone + Send + Sync + 'static> From<axum::Router<S>> for Router<S> {
    /// Wraps an untyped router. Its routes are not recorded or audited.
    fn from(inner: axum::Router<S>) -> Self {
        Router {
            inner,
            routes: Vec::new(),
        }
    }
}

impl<S: Clone + Send + Sync + 'static> Router<S> {
    /// An empty router.
    pub fn new() -> Self {
        Router {
            inner: axum::Router::new(),
            routes: Vec::new(),
        }
    }

    /// Adds a route; see `axum::Router::route`.
    pub fn route(mut self, path: &str, method_router: MethodRouter<S>) -> Self {
        for (method, handler) in &method_router.handlers {
            self.routes.push(RouteRecord {
                method: method.clone(),
                path: path.to_owned(),
                handler,
            });
        }
        self.inner = self.inner.route(path, method_router.inner);
        self
    }

    /// Merges another router; see `axum::Router::merge`.
    pub fn merge(mut self, other: impl Into<Router<S>>) -> Self {
        let other = other.into();
        self.routes.extend(other.routes);
        self.inner = self.inner.merge(other.inner);
        self
    }

    /// Nests another router under `path`; see `axum::Router::nest`.
    pub fn nest(mut self, path: &str, other: impl Into<Router<S>>) -> Self {
        let other = other.into();
        let prefix = path.trim_end_matches('/');
        self.routes.extend(other.routes.into_iter().map(|mut r| {
            r.path = format!("{prefix}{}", r.path);
            r
        }));
        self.inner = self.inner.nest(path, other.inner);
        self
    }

    /// Applies a layer to all routes; see `axum::Router::layer`.
    pub fn layer<L>(mut self, layer: L) -> Self
    where
        L: Layer<Route> + Clone + Send + Sync + 'static,
        L::Service: Service<Request> + Clone + Send + Sync + 'static,
        <L::Service as Service<Request>>::Response: IntoResponse + 'static,
        <L::Service as Service<Request>>::Error: Into<Infallible> + 'static,
        <L::Service as Service<Request>>::Future: Send + 'static,
    {
        self.inner = self.inner.layer(layer);
        self
    }

    /// Provides the state; see `axum::Router::with_state`.
    pub fn with_state<S2>(self, state: S) -> Router<S2> {
        Router {
            inner: self.inner.with_state(state),
            routes: self.routes,
        }
    }

    /// The inner `axum::Router`.
    pub fn into_axum(self) -> axum::Router<S> {
        self.inner
    }
}
