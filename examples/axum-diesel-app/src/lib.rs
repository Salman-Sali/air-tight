//! Example application for air-tight: axum, diesel and a Postgres database.
//!
//! The library holds the whole app so integration tests can build the router;
//! `main.rs` only wires configuration, seeding and the server.
#![deny(missing_docs)]

pub mod app;
pub mod auth;
pub mod authz;
pub mod config;
pub mod db;
pub mod error;
pub mod examples;
pub mod models;
pub mod requests;
pub mod routes;
#[allow(missing_docs)]
pub mod schema;
pub mod seed;
pub mod services;
pub mod state;

pub use app::{app, protected_routes, public_routes, router};
pub use state::{AppState, Client};

// Registers the permission, principal and resource kind types that
// `#[air_tight::permission]` refers to. Must be invoked once, at the crate root.
air_tight::setup!(
    perms = authz::MyPerms,
    user = auth::AuthenticatedUser,
    resources = authz::MyResources
);
