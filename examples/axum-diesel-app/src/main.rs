//! Runs the example server. See the README for the curl commands.

use axum_diesel_app::auth::AuthenticatedUser;
use axum_diesel_app::authz::{MyPerms, MyResources};
use axum_diesel_app::config::Config;
use axum_diesel_app::db::{Db, connect, migrate};
use axum_diesel_app::examples::curl_examples;
use axum_diesel_app::{AppState, router, seed};

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::from_env()?;

    let pool = connect(&config.database_url, 8)?;
    migrate(&mut *pool.get()?)?;

    let air_tight = air_tight::AirTight::<MyPerms, AuthenticatedUser>::builder(pool.clone())
        .resource_kinds::<MyResources>()
        .build()?;
    let state = AppState::new(Db::new(pool), air_tight.clone());
    seed::seed(&state).map_err(|err| format!("seeding failed: {err:?}"))?;

    let router = router(&state);
    // Panics at startup if any route is served by a handler that is not
    // protected by `#[air_tight::permission]` or `#[air_tight::unrestricted]`.
    air_tight.audit_routes(&router);

    let listener = tokio::net::TcpListener::bind(config.bind_addr).await?;
    println!("listening on http://{}", config.bind_addr);
    print!("{}", curl_examples(&format!("http://{}", config.bind_addr)));
    axum::serve(listener, router.with_state(state).into_axum()).await?;
    Ok(())
}
