use air_tight::PgPool;
use diesel::PgConnection;

use super::blocking;
use crate::error::ApiError;

/// The example's connection pool, with an async-friendly way to run queries.
#[derive(Clone)]
pub struct Db {
    pool: PgPool,
}

impl Db {
    /// Wraps a pool.
    pub fn new(pool: PgPool) -> Self {
        Db { pool }
    }

    /// The underlying pool.
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    /// Runs `work` with a pooled connection on a blocking thread.
    pub async fn run<T, F>(&self, work: F) -> Result<T, ApiError>
    where
        T: Send + 'static,
        F: FnOnce(&mut PgConnection) -> Result<T, ApiError> + Send + 'static,
    {
        let pool = self.pool.clone();
        blocking(move || {
            let mut conn = pool.get()?;
            work(&mut conn)
        })
        .await
    }
}
