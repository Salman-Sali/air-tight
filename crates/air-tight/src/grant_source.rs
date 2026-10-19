use crate::RoleId;

/// Where an effective grant comes from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GrantSource {
    /// Granted directly to the user.
    Direct,
    /// Inherited from a role assignment.
    Role {
        /// The role's id.
        id: RoleId,
        /// The role's name.
        name: String,
    },
}
