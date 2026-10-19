/// Outcome of [`sync_catalog`](crate::sync_catalog).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CatalogSyncReport {
    /// Number of permission keys upserted.
    pub permissions: usize,
    /// Number of resource kinds upserted.
    pub resource_kinds: usize,
    /// Permission keys present in the database but absent from code.
    pub stale_permissions: Vec<String>,
}
