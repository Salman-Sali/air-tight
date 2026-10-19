use std::fmt::Debug;
use std::hash::Hash;

use crate::ResourceKindKey;

/// Enum of resource kinds (unit variants only).
pub trait ResourceKind: Copy + Eq + Hash + Debug + Send + Sync + 'static {
    /// The key of this kind.
    fn key(&self) -> ResourceKindKey;
    /// Every kind.
    fn all() -> &'static [Self];
    /// Parses a key back into a kind.
    fn from_key(key: &str) -> Option<Self>;
}
