use diesel::PgConnection;
use diesel::r2d2::{ConnectionManager, Pool};

/// The r2d2 pool of Postgres connections that [`crate::AirTight`] runs on.
pub type PgPool = Pool<ConnectionManager<PgConnection>>;
