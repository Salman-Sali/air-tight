use crate::{PermissionKey, RoleId};

/// A role with its permission keys, as returned by `list_roles`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RoleInfo {
    /// The role's id.
    pub id: RoleId,
    /// The role's unique name.
    pub name: String,
    /// Optional free-text description.
    pub description: Option<String>,
    /// The keys in the role, sorted; may include wildcards.
    pub permissions: Vec<PermissionKey>,
}
