//! Route table and application builders.
use air_tight::{Router, routing};
use axum::middleware::from_fn_with_state;

use crate::auth::authenticate;
use crate::routes;
use crate::state::AppState;

/// Routes that need no authentication.
pub fn public_routes() -> Router<AppState> {
    Router::new().route("/health", routing::get(routes::health))
}

/// Routes behind the authentication middleware.
pub fn protected_routes() -> Router<AppState> {
    Router::new()
        .route("/workspaces", routing::get(routes::list_workspaces))
        .route("/workspaces/sql", routing::get(routes::list_workspaces_sql))
        .route(
            "/workspaces/{id}",
            routing::get(routes::get_workspace).patch(routes::update_workspace),
        )
        .route("/invoices", routing::post(routes::create_invoice))
        .route("/invoices/move", routing::post(routes::move_invoice))
        .route("/invoices/{id}", routing::get(routes::get_invoice))
        .route("/billing", routing::get(routes::billing))
        .route("/billing/readonly", routing::get(routes::billing_readonly))
        .route("/admin/roles", routing::post(routes::admin_roles))
        .route("/admin/assign", routing::post(routes::admin_assign))
        .route("/admin/grant", routing::post(routes::admin_grant))
}

/// The whole typed router, with authentication applied to the protected
/// routes. Pass it to `AirTight::audit_routes` before serving.
pub fn router(state: &AppState) -> Router<AppState> {
    let authenticated =
        protected_routes().layer(from_fn_with_state(state.db.clone(), authenticate));
    public_routes().merge(authenticated)
}

/// The finished axum application.
pub fn app(state: AppState) -> axum::Router {
    router(&state).with_state(state).into_axum()
}
