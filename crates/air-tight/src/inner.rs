use std::sync::Arc;

use diesel::PgConnection;

use crate::{CatalogSyncReport, Error, GrantCache, NotifyChannel, PgPool};

/// Catalog sync entry point monomorphised over the app's types at build time.
pub(crate) type SyncFn = fn(&mut PgConnection) -> Result<CatalogSyncReport, Error>;

/// Shared state of an [`crate::AirTight`] client.
pub(crate) struct Inner {
    pub(crate) pool: PgPool,
    pub(crate) cache: Arc<dyn GrantCache>,
    pub(crate) channel: NotifyChannel,
    pub(crate) sync: SyncFn,
}
