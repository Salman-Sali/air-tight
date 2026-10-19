use air_tight_core::{PermissionCategory, Principal};

use crate::AirTight;
use crate::registry;
use crate::router::Router;

impl<P: PermissionCategory, U: Principal> AirTight<P, U> {
    /// Checks every route recorded by the typed `router` against the handler
    /// registry filled by `#[air_tight::permission]` and
    /// `#[air_tight::unrestricted]`.
    ///
    /// # Panics
    ///
    /// Panics, naming the method and path, if a route's handler is not
    /// registered (for example a hand-built `Guarded`). Call it in `main`
    /// before serving. Routes merged from a plain `axum::Router` are not
    /// recorded and therefore not checked.
    pub fn audit_routes<S>(&self, router: &Router<S>) {
        registry::audit_routes(router);
    }
}
