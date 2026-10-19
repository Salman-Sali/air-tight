//! Shared helpers for database-backed integration tests.
#![allow(dead_code)]

use air_tight::run_migrations;
use diesel::prelude::*;

/// Connects to the test database and migrates it, or returns `None` to skip.
pub fn connect() -> Option<PgConnection> {
    let Ok(url) = std::env::var("AIR_TIGHT_TEST_DATABASE_URL") else {
        eprintln!("skipping: AIR_TIGHT_TEST_DATABASE_URL is not set");
        return None;
    };
    let mut conn = PgConnection::establish(&url).expect("connect to test database");
    run_migrations(&mut conn).expect("run migrations");
    Some(conn)
}

/// A unique string with the given prefix.
pub fn unique(prefix: &str) -> String {
    format!("{prefix}-{}", uuid::Uuid::new_v4())
}
