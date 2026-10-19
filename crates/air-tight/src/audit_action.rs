/// The `action` column values of management audit rows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AuditAction {
    RoleCreated,
    RoleDeleted,
    RolePermissionsSet,
    RoleAssigned,
    RoleRevoked,
    GrantAdded,
    GrantRevoked,
    PermissionRenamed,
}

impl AuditAction {
    /// The stored string.
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            AuditAction::RoleCreated => "role_created",
            AuditAction::RoleDeleted => "role_deleted",
            AuditAction::RolePermissionsSet => "role_permissions_set",
            AuditAction::RoleAssigned => "role_assigned",
            AuditAction::RoleRevoked => "role_revoked",
            AuditAction::GrantAdded => "grant_added",
            AuditAction::GrantRevoked => "grant_revoked",
            AuditAction::PermissionRenamed => "permission_renamed",
        }
    }
}
