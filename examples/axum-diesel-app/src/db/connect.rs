use air_tight::PgPool;
use diesel::PgConnection;
use diesel::r2d2::{ConnectionManager, Pool};

/// Builds a connection pool for `database_url`.
///
/// air-tight keeps one connection for its listener, so `max_size` must be at
/// least two.
pub fn connect(database_url: &str, max_size: u32) -> Result<PgPool, diesel::r2d2::PoolError> {
    Pool::builder()
        .max_size(max_size)
        .build(ConnectionManager::<PgConnection>::new(database_url))
}
