use air_tight_core::{PermissionCategory, PermissionKey, Principal};
use diesel::prelude::*;
use serde_json::json;

use crate::air_tight::AirTight;
use crate::audit_action::AuditAction;
use crate::audit_entry::AuditEntry;
use crate::invalidation::Invalidation;
use crate::schema::{role_permissions, roles, user_roles};
use crate::validate_key::ensure_known_keys;
use crate::{Error, RoleId, RoleName};

/// Above this many affected users a role deletion broadcasts `all` instead of
/// one `user:<id>` message each.
const MAX_USER_BROADCAST: usize = 100;

impl<P: PermissionCategory, U: Principal> AirTight<P, U> {
    /// Creates a role and returns its id.
    ///
    /// `actor` is the id of the user performing the change, recorded in the
    /// audit log (`role_created`); pass `None` for system actions. The name must
    /// be unique: a duplicate fails with `Error::Db`.
    pub fn create_role(
        &self,
        actor: Option<&str>,
        name: &RoleName,
        description: Option<&str>,
    ) -> Result<RoleId, Error> {
        self.inner.mutate(|conn| {
            let id = RoleId::new();
            diesel::insert_into(roles::table)
                .values((
                    roles::id.eq(id.as_uuid()),
                    roles::name.eq(name.as_str()),
                    roles::description.eq(description),
                ))
                .execute(conn)?;
            AuditEntry::new(actor, AuditAction::RoleCreated)
                .role(id)
                .detail(json!({ "name": name.as_str(), "description": description }))
                .write(conn)?;
            // A new role has no holders yet, so there is nothing to invalidate.
            Ok((id, Vec::new()))
        })
    }

    /// Deletes a role together with its permissions and assignments.
    ///
    /// Audited as `role_deleted` by `actor`. Deleting a role that does not
    /// exist is an `Ok` no-op and writes no audit row. Affected users are
    /// collected before the cascade removes their assignments, then invalidated
    /// locally and announced as `user:<id>` messages (or `all` for very large
    /// roles).
    pub fn delete_role(&self, actor: Option<&str>, role: RoleId) -> Result<(), Error> {
        self.inner.mutate(|conn| {
            let users: Vec<String> = user_roles::table
                .filter(user_roles::role_id.eq(role.as_uuid()))
                .select(user_roles::user_id)
                .distinct()
                .load(conn)?;
            let name: Option<String> = roles::table
                .find(role.as_uuid())
                .select(roles::name)
                .first(conn)
                .optional()?;
            let deleted = diesel::delete(roles::table.find(role.as_uuid())).execute(conn)?;
            if deleted == 0 {
                return Ok(((), Vec::new()));
            }
            AuditEntry::new(actor, AuditAction::RoleDeleted)
                .role(role)
                .detail(json!({ "name": name, "affected_users": users.len() }))
                .write(conn)?;
            let invalidations = if users.len() > MAX_USER_BROADCAST {
                vec![Invalidation::All]
            } else {
                users.into_iter().map(Invalidation::User).collect()
            };
            Ok(((), invalidations))
        })
    }

    /// Replaces the whole permission set of a role.
    ///
    /// Every key must exist in the catalog or be a well-formed wildcard (`*` or
    /// `<Category>::*`); otherwise nothing changes and `Error::Catalog` names
    /// the offending key. Audited as `role_permissions_set` by `actor`; holders
    /// of the role are invalidated through a `role:<uuid>` message.
    pub fn set_role_permissions(
        &self,
        actor: Option<&str>,
        role: RoleId,
        permissions: &[PermissionKey],
    ) -> Result<(), Error> {
        self.inner.mutate(|conn| {
            ensure_known_keys(conn, permissions)?;
            let exists: Option<RoleId> = roles::table
                .find(role.as_uuid())
                .select(roles::id)
                .first(conn)
                .optional()?
                .map(RoleId::from_uuid);
            if exists.is_none() {
                return Err(Error::Catalog(format!("unknown role {role}")));
            }
            diesel::delete(
                role_permissions::table.filter(role_permissions::role_id.eq(role.as_uuid())),
            )
            .execute(conn)?;
            let mut keys: Vec<&str> = permissions.iter().map(PermissionKey::as_str).collect();
            keys.sort_unstable();
            keys.dedup();
            let rows: Vec<_> = keys
                .iter()
                .map(|k| {
                    (
                        role_permissions::role_id.eq(role.as_uuid()),
                        role_permissions::permission_key.eq(*k),
                    )
                })
                .collect();
            if !rows.is_empty() {
                diesel::insert_into(role_permissions::table)
                    .values(&rows)
                    .execute(conn)?;
            }
            AuditEntry::new(actor, AuditAction::RolePermissionsSet)
                .role(role)
                .detail(json!({ "permissions": keys }))
                .write(conn)?;
            Ok(((), vec![Invalidation::Role(role)]))
        })
    }

    /// Invalidates the cached grants of every user holding `role`.
    ///
    /// Precise: it looks the holders up in `user_roles` and invalidates each
    /// one, rather than dropping the whole cache.
    pub fn invalidate_role(&self, role: RoleId) -> Result<(), Error> {
        let mut conn = self.inner.pool.get()?;
        self.inner.invalidate_role(&mut conn, role)
    }
}
