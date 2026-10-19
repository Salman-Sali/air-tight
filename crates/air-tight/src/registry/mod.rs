//! Process-wide registry of handlers processed by `#[air_tight::permission]`
//! and `#[air_tight::unrestricted]`, collected with `inventory`.

mod handler_entry;
mod handler_kind;

pub use handler_entry::HandlerEntry;
pub use handler_kind::HandlerKind;

inventory::collect!(HandlerEntry);

/// Every registered handler, in no particular order.
pub fn entries() -> impl Iterator<Item = &'static HandlerEntry> {
    inventory::iter::<HandlerEntry>.into_iter()
}

/// Finds the entry for a fully qualified handler path such as
/// `my_app::routes::get_workspace`.
pub fn find(path: &str) -> Option<&'static HandlerEntry> {
    entries().find(|e| e.matches(path))
}

/// Checks every route recorded by a typed router against the registry.
/// [`AirTight::audit_routes`](crate::AirTight::audit_routes) delegates here.
///
/// # Panics
///
/// Panics, naming the method, path and handler, on the first route whose
/// handler is not registered.
pub fn audit_routes<S>(router: &crate::router::Router<S>) {
    for route in router.routes() {
        if find(route.handler).is_none() {
            panic!(
                "air_tight: route `{} {}` is served by `{}`, which is not registered by \
                 #[air_tight::permission] or #[air_tight::unrestricted]",
                route.method, route.path, route.handler
            );
        }
        tracing::debug!(method = %route.method, path = %route.path, handler = route.handler, "route audited");
    }
}
