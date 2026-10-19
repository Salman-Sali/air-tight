use std::sync::Arc;
use std::time::Duration;

use moka::sync::Cache;

use crate::{GrantCache, GrantSet};

/// The default [`GrantCache`], backed by a moka cache.
///
/// Entries expire after `idle_ttl` without access (every `get` refreshes the
/// timer). Capacity is measured in approximate bytes: each entry weighs its key
/// length plus [`GrantSet::approx_bytes`].
#[derive(Clone)]
pub struct MokaGrantCache {
    cache: Cache<String, Arc<GrantSet>>,
}

impl MokaGrantCache {
    /// Creates a cache with the given idle time-to-live and capacity in bytes.
    pub fn new(idle_ttl: Duration, max_bytes: u64) -> Self {
        let cache = Cache::builder()
            .max_capacity(max_bytes)
            .time_to_idle(idle_ttl)
            .weigher(|key: &String, set: &Arc<GrantSet>| {
                u32::try_from(key.len())
                    .unwrap_or(u32::MAX)
                    .saturating_add(set.approx_bytes())
            })
            .build();
        MokaGrantCache { cache }
    }

    /// Runs moka's deferred maintenance (evictions, expirations) now.
    ///
    /// Moka applies some housekeeping lazily, so call this before asserting on
    /// entry counts.
    pub fn run_pending_tasks(&self) {
        self.cache.run_pending_tasks();
    }

    /// Approximate number of entries; see [`MokaGrantCache::run_pending_tasks`].
    pub fn entry_count(&self) -> u64 {
        self.cache.entry_count()
    }
}

impl GrantCache for MokaGrantCache {
    fn get(&self, user_id: &str) -> Option<Arc<GrantSet>> {
        self.cache.get(user_id)
    }

    fn insert(&self, user_id: String, set: Arc<GrantSet>) {
        self.cache.insert(user_id, set);
    }

    fn invalidate(&self, user_id: &str) {
        self.cache.invalidate(user_id);
    }

    fn invalidate_all(&self) {
        self.cache.invalidate_all();
    }
}
