//! Settings read from the environment.
use std::net::SocketAddr;

/// Database used when `DATABASE_URL` is not set: the one created by the
/// repository's `docker-compose.yml`.
pub const DEFAULT_DATABASE_URL: &str =
    "postgres://postgres:password@localhost:55432/air_tight_example";

/// Address used when `BIND_ADDR` is not set.
pub const DEFAULT_BIND_ADDR: &str = "127.0.0.1:3000";

/// Settings read from the environment.
#[derive(Clone, Debug)]
pub struct Config {
    /// Postgres connection URL (`DATABASE_URL`).
    pub database_url: String,
    /// Address to listen on (`BIND_ADDR`).
    pub bind_addr: SocketAddr,
}

impl Config {
    /// Reads `DATABASE_URL` and `BIND_ADDR`, falling back to the defaults.
    pub fn from_env() -> Result<Self, String> {
        let database_url =
            std::env::var("DATABASE_URL").unwrap_or_else(|_| DEFAULT_DATABASE_URL.to_owned());
        let bind = std::env::var("BIND_ADDR").unwrap_or_else(|_| DEFAULT_BIND_ADDR.to_owned());
        let bind_addr = bind
            .parse()
            .map_err(|err| format!("invalid BIND_ADDR `{bind}`: {err}"))?;
        Ok(Config {
            database_url,
            bind_addr,
        })
    }
}
