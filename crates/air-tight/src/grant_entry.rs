use crate::{GrantSource, PermissionKey, ResourceScope};

/// One permission key a user holds, with its scope and origin.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GrantEntry {
    /// The granted key; may be a wildcard such as `*` or `Workspace::*`.
    pub key: PermissionKey,
    /// The resource the grant is limited to, or `None` for a global grant.
    pub scope: Option<ResourceScope>,
    /// Whether the grant is direct or comes through a role.
    pub source: GrantSource,
}
