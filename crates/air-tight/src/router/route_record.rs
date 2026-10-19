use axum::http::Method;

/// One `(method, path, handler)` triple recorded by the typed router, used
/// by [`AirTight::audit_routes`](crate::AirTight::audit_routes).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RouteRecord {
    /// HTTP method.
    pub method: Method,
    /// Route path, including any `nest` prefix.
    pub path: String,
    /// Fully qualified handler path from `std::any::type_name`, generics stripped.
    pub handler: &'static str,
}
