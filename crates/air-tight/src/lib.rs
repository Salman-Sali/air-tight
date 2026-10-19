//! Runtime crate for air-tight. Re-exports the core types and the macros.
#![deny(missing_docs)]

mod air_tight;
mod builder;
mod catalog_sync_report;
mod error;
mod evaluate;
mod grant_cache;
mod grant_set;
mod granted;
mod inner;
mod intern_table;
mod interner;
mod key_id;
mod kind_id;
mod load_grant_set;
mod migrations;
mod moka_grant_cache;
mod no_kinds;
mod notify_channel;
mod pg_pool;
mod role_id;
#[allow(missing_docs)]
pub mod schema;
mod sync_catalog;

pub use air_tight::AirTight;
pub use air_tight_core::*;
pub use air_tight_macros::{IdentifyResources, Permission, PermissionCategory, ResourceKind};
pub use builder::Builder;
pub use catalog_sync_report::CatalogSyncReport;
pub use error::Error;
pub use evaluate::evaluate;
pub use grant_cache::GrantCache;
pub use grant_set::GrantSet;
pub use granted::Granted;
pub use interner::Interner;
pub use key_id::KeyId;
pub use kind_id::KindId;
pub use load_grant_set::load_grant_set;
pub use migrations::{migration_sql, run_migrations};
pub use moka_grant_cache::MokaGrantCache;
pub use no_kinds::NoKinds;
pub use notify_channel::NotifyChannel;
pub use pg_pool::PgPool;
pub use role_id::RoleId;
pub use sync_catalog::sync_catalog;
