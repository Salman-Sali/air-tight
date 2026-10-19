use crate::{PermissionKey, ResourceKindKey};

/// A catalog row, as returned by `list_permissions`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PermissionInfo {
    /// The permission key, e.g. `Workspace::Update`.
    pub key: PermissionKey,
    /// The category part of the key.
    pub category: String,
    /// The action part of the key.
    pub action: String,
    /// The resource kind the category is bound to, if any.
    pub resource_kind: Option<ResourceKindKey>,
    /// Optional free-text description.
    pub description: Option<String>,
}
