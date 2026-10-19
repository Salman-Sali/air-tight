use std::sync::Arc;

use crate::GrantSet;

/// A cache of per-user grant sets.
///
/// Implementations must be cheap to call from many threads at once. The default
/// is [`crate::MokaGrantCache`]; supply your own with
/// [`crate::Builder::cache`].
pub trait GrantCache: Send + Sync {
    /// Returns the cached grant set of `user_id`, if present and not expired.
    fn get(&self, user_id: &str) -> Option<Arc<GrantSet>>;
    /// Stores the grant set of `user_id`, replacing any previous entry.
    fn insert(&self, user_id: String, set: Arc<GrantSet>);
    /// Drops the entry of `user_id`, if any.
    fn invalidate(&self, user_id: &str);
    /// Drops every entry.
    fn invalidate_all(&self);
}
