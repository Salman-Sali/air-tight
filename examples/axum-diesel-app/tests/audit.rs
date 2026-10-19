//! The startup route audit. None of these tests need a database.

use air_tight::{Router, registry, routing};
use axum_diesel_app::{AppState, protected_routes, public_routes};

/// A hand-built `Guarded` that skips `#[air_tight::permission]`, so it is not
/// in the handler registry.
fn sneaky<S: Clone + Send + Sync + 'static>()
-> air_tight::Guarded<impl axum::handler::Handler<((),), S>> {
    air_tight::Guarded::__new(|| async { "sneaky" })
}

#[test]
fn the_real_routes_pass_the_audit() {
    let routes: Router<AppState> = public_routes().merge(protected_routes());
    assert_eq!(routes.routes().len(), 13);
    registry::audit_routes(&routes);
}

#[test]
fn every_route_handler_is_registered() {
    let routes: Router<AppState> = public_routes().merge(protected_routes());
    for route in routes.routes() {
        let entry = registry::find(route.handler)
            .unwrap_or_else(|| panic!("{} {} is not registered", route.method, route.path));
        println!("{} {} -> {}", route.method, route.path, entry.condition);
    }
}

#[test]
#[should_panic(expected = "route `GET /sneaky` is served by")]
fn audit_panics_on_an_unregistered_route() {
    let routes: Router<AppState> = public_routes()
        .merge(protected_routes())
        .route("/sneaky", routing::get(sneaky));
    registry::audit_routes(&routes);
}
