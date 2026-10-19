use air_tight_core::{PermissionCategory, PermissionKey, Principal, ResourceScope};
use diesel::PgExpressionMethods;
use diesel::prelude::*;
use serde_json::json;

use crate::air_tight::AirTight;
use crate::audit_action::AuditAction;
use crate::audit_entry::AuditEntry;
use crate::invalidation::Invalidation;
use crate::schema::{roles, user_permissions, user_roles};
use crate::validate_key::ensure_known_keys;
use crate::{Error, RoleId};

#[derive(Insertable)]
#[diesel(table_name = user_roles)]
struct NewUserRole<'a> {
    user_id: &'a str,
    role_id: uuid::Uuid,
    scope_kind: Option<&'a str>,
    scope_id: Option<&'a str>,
}

#[derive(Insertable)]
#[diesel(table_name = user_permissions)]
struct NewUserPermission<'a> {
    user_id: &'a str,
    permission_key: &'a str,
    scope_kind: Option<&'a str>,
    scope_id: Option<&'a str>,
}

/// The invalidations after a change that affected `changed` rows of one user.
fn for_user(changed: usize, user_id: &str) -> Vec<Invalidation> {
    if changed == 0 {
        Vec::new()
    } else {
        vec![Invalidation::User(user_id.to_owned())]
    }
}

impl<P: PermissionCategory, U: Principal> AirTight<P, U> {
    /// Assigns a role to a user, globally or within one resource `scope`.
    ///
    /// `actor` is recorded in the audit log (`role_assigned`). Idempotent: if
    /// the assignment already exists nothing changes and no audit row or
    /// notification is written. An unknown role fails with `Error::Db`.
    pub fn assign_role(
        &self,
        actor: Option<&str>,
        user_id: &str,
        role: RoleId,
        scope: Option<ResourceScope>,
    ) -> Result<(), Error> {
        self.inner.mutate(|conn| {
            let row = NewUserRole {
                user_id,
                role_id: role.as_uuid(),
                scope_kind: scope.as_ref().map(|s| s.kind.as_str()),
                scope_id: scope.as_ref().map(|s| s.id.as_str()),
            };
            let added = diesel::insert_into(user_roles::table)
                .values(&row)
                .on_conflict_do_nothing()
                .execute(conn)?;
            if added > 0 {
                AuditEntry::new(actor, AuditAction::RoleAssigned)
                    .target_user(user_id)
                    .role(role)
                    .scope(scope.as_ref())
                    .write(conn)?;
            }
            Ok(((), for_user(added, user_id)))
        })
    }

    /// Removes a role assignment (the same `scope` it was assigned with).
    ///
    /// `actor` is recorded in the audit log (`role_revoked`). Revoking an
    /// assignment that does not exist is an `Ok` no-op: no audit row and no
    /// notification, because nothing changed.
    pub fn revoke_role(
        &self,
        actor: Option<&str>,
        user_id: &str,
        role: RoleId,
        scope: Option<ResourceScope>,
    ) -> Result<(), Error> {
        self.inner.mutate(|conn| {
            let kind = scope.as_ref().map(|s| s.kind.as_str());
            let id = scope.as_ref().map(|s| s.id.as_str());
            let removed = diesel::delete(
                user_roles::table
                    .filter(user_roles::user_id.eq(user_id))
                    .filter(user_roles::role_id.eq(role.as_uuid()))
                    .filter(user_roles::scope_kind.is_not_distinct_from(kind))
                    .filter(user_roles::scope_id.is_not_distinct_from(id)),
            )
            .execute(conn)?;
            if removed > 0 {
                let name: Option<String> = roles::table
                    .find(role.as_uuid())
                    .select(roles::name)
                    .first(conn)
                    .optional()?;
                AuditEntry::new(actor, AuditAction::RoleRevoked)
                    .target_user(user_id)
                    .role(role)
                    .scope(scope.as_ref())
                    .detail(json!({ "role_name": name }))
                    .write(conn)?;
            }
            Ok(((), for_user(removed, user_id)))
        })
    }

    /// Grants a permission key directly to a user, globally or within `scope`.
    ///
    /// The key must be in the catalog or a well-formed wildcard, else
    /// `Error::Catalog`. `actor` is recorded in the audit log (`grant_added`).
    /// Idempotent: an existing identical grant changes nothing and writes no
    /// audit row or notification.
    pub fn grant(
        &self,
        actor: Option<&str>,
        user_id: &str,
        key: PermissionKey,
        scope: Option<ResourceScope>,
    ) -> Result<(), Error> {
        self.inner.mutate(|conn| {
            ensure_known_keys(conn, [&key])?;
            let row = NewUserPermission {
                user_id,
                permission_key: key.as_str(),
                scope_kind: scope.as_ref().map(|s| s.kind.as_str()),
                scope_id: scope.as_ref().map(|s| s.id.as_str()),
            };
            let added = diesel::insert_into(user_permissions::table)
                .values(&row)
                .on_conflict_do_nothing()
                .execute(conn)?;
            if added > 0 {
                AuditEntry::new(actor, AuditAction::GrantAdded)
                    .target_user(user_id)
                    .permission(key.as_str())
                    .scope(scope.as_ref())
                    .write(conn)?;
            }
            Ok(((), for_user(added, user_id)))
        })
    }

    /// Removes a direct grant (the same key and `scope` it was granted with).
    ///
    /// `actor` is recorded in the audit log (`grant_revoked`). Revoking a grant
    /// that does not exist is an `Ok` no-op: no audit row and no notification.
    /// The key is not validated, so grants of keys that code no longer defines
    /// can still be removed.
    pub fn revoke(
        &self,
        actor: Option<&str>,
        user_id: &str,
        key: PermissionKey,
        scope: Option<ResourceScope>,
    ) -> Result<(), Error> {
        self.inner.mutate(|conn| {
            let kind = scope.as_ref().map(|s| s.kind.as_str());
            let id = scope.as_ref().map(|s| s.id.as_str());
            let removed = diesel::delete(
                user_permissions::table
                    .filter(user_permissions::user_id.eq(user_id))
                    .filter(user_permissions::permission_key.eq(key.as_str()))
                    .filter(user_permissions::scope_kind.is_not_distinct_from(kind))
                    .filter(user_permissions::scope_id.is_not_distinct_from(id)),
            )
            .execute(conn)?;
            if removed > 0 {
                AuditEntry::new(actor, AuditAction::GrantRevoked)
                    .target_user(user_id)
                    .permission(key.as_str())
                    .scope(scope.as_ref())
                    .write(conn)?;
            }
            Ok(((), for_user(removed, user_id)))
        })
    }
}
