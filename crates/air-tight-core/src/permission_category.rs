use std::fmt::Debug;
use std::hash::Hash;

use crate::{PermissionKey, ResourceKindKey};

/// A set of category/action combinations making up the app's permissions.
pub trait PermissionCategory: Copy + Eq + Hash + Debug + Send + Sync + 'static {
    /// The full key, `<Category>::<Action>`.
    fn key(&self) -> PermissionKey;
    /// The category segment of the key (before `::`).
    fn category_name(&self) -> &'static str;
    /// Every category/action combination.
    fn all() -> &'static [Self];
    /// The resource kind this category is bound to, if any.
    fn resource_kind(&self) -> Option<ResourceKindKey>;
    /// Parses a full key back into a permission.
    fn from_key(key: &str) -> Option<Self>;
}
