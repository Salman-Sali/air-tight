use air_tight_core::{PermissionCategory, PermissionKey, Principal, is_well_formed_wildcard};
use diesel::prelude::*;
use diesel::sql_query;
use diesel::sql_types::Text;
use serde_json::json;

use crate::Error;
use crate::air_tight::AirTight;
use crate::audit_action::AuditAction;
use crate::audit_entry::AuditEntry;
use crate::invalidation::Invalidation;

impl<P: PermissionCategory, U: Principal> AirTight<P, U> {
    /// Renames a permission key everywhere in the database.
    ///
    /// Rewrites `role_permissions`, `user_permissions` and the catalog row in
    /// one transaction. Where the new key already exists (for example because
    /// `sync_catalog` added it before the rename), duplicates that would violate
    /// uniqueness are dropped and the rest are moved, so the result is a merge.
    /// `new` must look like `Category::Action` or be a well-formed wildcard.
    /// Audited as `permission_renamed` by `actor` (the old key is in
    /// `permission_key`, the new one in `detail`); every client is told to drop
    /// its whole cache through an `all` message.
    pub fn rename_permission(
        &self,
        actor: Option<&str>,
        old: &PermissionKey,
        new: &PermissionKey,
    ) -> Result<(), Error> {
        if old == new {
            return Err(Error::Catalog(format!("cannot rename `{old}` to itself")));
        }
        if !is_well_formed_wildcard(new.as_str()) && new.as_str().split_once("::").is_none() {
            return Err(Error::Catalog(format!(
                "invalid new permission key `{new}`: expected Category::Action or a wildcard"
            )));
        }
        self.inner.mutate(|conn| {
            let (o, n) = (old.as_str(), new.as_str());
            sql_query(
                "DELETE FROM air_tight.role_permissions r WHERE r.permission_key = $1 \
                 AND EXISTS (SELECT 1 FROM air_tight.role_permissions x \
                             WHERE x.role_id = r.role_id AND x.permission_key = $2)",
            )
            .bind::<Text, _>(o)
            .bind::<Text, _>(n)
            .execute(conn)?;
            sql_query("UPDATE air_tight.role_permissions SET permission_key = $2 WHERE permission_key = $1")
                .bind::<Text, _>(o)
                .bind::<Text, _>(n)
                .execute(conn)?;
            sql_query(
                "DELETE FROM air_tight.user_permissions u WHERE u.permission_key = $1 \
                 AND EXISTS (SELECT 1 FROM air_tight.user_permissions x \
                             WHERE x.permission_key = $2 AND x.user_id = u.user_id \
                               AND x.scope_kind IS NOT DISTINCT FROM u.scope_kind \
                               AND x.scope_id IS NOT DISTINCT FROM u.scope_id)",
            )
            .bind::<Text, _>(o)
            .bind::<Text, _>(n)
            .execute(conn)?;
            sql_query("UPDATE air_tight.user_permissions SET permission_key = $2 WHERE permission_key = $1")
                .bind::<Text, _>(o)
                .bind::<Text, _>(n)
                .execute(conn)?;
            // Catalog row: if the new row exists the old one is just dropped,
            // otherwise it is moved and its category and action re-derived.
            sql_query(
                "DELETE FROM air_tight.permissions p WHERE p.key = $1 \
                 AND EXISTS (SELECT 1 FROM air_tight.permissions x WHERE x.key = $2)",
            )
            .bind::<Text, _>(o)
            .bind::<Text, _>(n)
            .execute(conn)?;
            let (category, action) = n.split_once("::").unwrap_or((n, ""));
            sql_query(
                "UPDATE air_tight.permissions SET key = $2, category = $3, action = $4 WHERE key = $1",
            )
            .bind::<Text, _>(o)
            .bind::<Text, _>(n)
            .bind::<Text, _>(category)
            .bind::<Text, _>(action)
            .execute(conn)?;
            AuditEntry::new(actor, AuditAction::PermissionRenamed)
                .permission(o)
                .detail(json!({ "new": n }))
                .write(conn)?;
            Ok(((), vec![Invalidation::All]))
        })
    }
}
