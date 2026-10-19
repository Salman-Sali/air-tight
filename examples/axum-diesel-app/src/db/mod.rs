//! Database access: the pool wrapper, connecting and the example's migrations.

mod blocking;
mod connect;
mod database;
mod migrate;

pub use blocking::blocking;
pub use connect::connect;
pub use database::Db;
pub use migrate::migrate;
