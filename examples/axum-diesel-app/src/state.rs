//! Router state and the `FromRef` impls air-tight needs.
use air_tight::AirTight;
use axum::extract::FromRef;

use crate::auth::AuthenticatedUser;
use crate::authz::MyPerms;
use crate::db::Db;

/// The air-tight client of the example.
pub type Client = AirTight<MyPerms, AuthenticatedUser>;

/// The router state.
#[derive(Clone)]
pub struct AppState {
    /// The example's database.
    pub db: Db,
    /// The authorization client.
    pub air_tight: Client,
}

impl AppState {
    /// Bundles the database and the authorization client.
    pub fn new(db: Db, air_tight: Client) -> Self {
        AppState { db, air_tight }
    }
}

// `#[air_tight::permission]` extracts `State<AirTight<..>>`, which needs this.
impl FromRef<AppState> for Client {
    fn from_ref(state: &AppState) -> Self {
        state.air_tight.clone()
    }
}

impl FromRef<AppState> for Db {
    fn from_ref(state: &AppState) -> Self {
        state.db.clone()
    }
}
