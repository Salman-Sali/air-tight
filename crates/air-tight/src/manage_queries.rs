use air_tight_core::{
    PermissionCategory, PermissionKey, Principal, ResourceKindKey, ResourceScope,
};
use diesel::prelude::*;
use diesel::sql_query;
use diesel::sql_types::Text;

use crate::air_tight::AirTight;
use crate::schema::{permissions, role_permissions, roles, user_permissions, user_roles};
use crate::{Error, GrantEntry, GrantReport, GrantSource, PermissionInfo, RoleId, RoleInfo};

#[derive(QueryableByName)]
struct UserIdRow {
    #[diesel(sql_type = Text)]
    user_id: String,
}

type DirectRow = (String, Option<String>, Option<String>);
type PermissionRow = (String, String, String, Option<String>, Option<String>);
type RoleRow = (String, Option<String>, Option<String>, uuid::Uuid, String);

fn scope_of(kind: Option<String>, id: Option<String>) -> Option<ResourceScope> {
    match (kind, id) {
        (Some(kind), Some(id)) => Some(ResourceScope::new(ResourceKindKey::new(kind), id)),
        _ => None,
    }
}

/// Orders entries by key, then scope, then source, for stable reports.
fn sort_key(entry: &GrantEntry) -> (String, String, String, String) {
    let (kind, id) = entry
        .scope
        .as_ref()
        .map(|s| (s.kind.to_string(), s.id.clone()))
        .unwrap_or_default();
    let source = match &entry.source {
        GrantSource::Direct => String::new(),
        GrantSource::Role { name, .. } => name.clone(),
    };
    (entry.key.to_string(), kind, id, source)
}

impl<P: PermissionCategory, U: Principal> AirTight<P, U> {
    /// Lists everything `user_id` holds, straight from the database.
    ///
    /// Bypasses the cache, so it always shows the current truth. Each entry says
    /// whether it is a direct grant or comes from a role. Entries are sorted by
    /// key, then scope.
    pub fn effective_grants(&self, user_id: &str) -> Result<GrantReport, Error> {
        let mut conn = self.inner.pool.get()?;
        let direct: Vec<DirectRow> = user_permissions::table
            .filter(user_permissions::user_id.eq(user_id))
            .select((
                user_permissions::permission_key,
                user_permissions::scope_kind,
                user_permissions::scope_id,
            ))
            .load(&mut conn)?;
        let via_roles: Vec<RoleRow> = user_roles::table
            .inner_join(roles::table)
            .inner_join(
                role_permissions::table.on(role_permissions::role_id.eq(user_roles::role_id)),
            )
            .filter(user_roles::user_id.eq(user_id))
            .select((
                role_permissions::permission_key,
                user_roles::scope_kind,
                user_roles::scope_id,
                roles::id,
                roles::name,
            ))
            .load(&mut conn)?;

        let entries = direct
            .into_iter()
            .map(|(key, kind, id)| GrantEntry {
                key: PermissionKey::new(key),
                scope: scope_of(kind, id),
                source: GrantSource::Direct,
            })
            .chain(
                via_roles
                    .into_iter()
                    .map(|(key, kind, id, role, name)| GrantEntry {
                        key: PermissionKey::new(key),
                        scope: scope_of(kind, id),
                        source: GrantSource::Role {
                            id: RoleId::from_uuid(role),
                            name,
                        },
                    }),
            );
        let mut report = GrantReport::default();
        for entry in entries {
            if entry.scope.is_some() {
                report.scoped.push(entry);
            } else {
                report.global.push(entry);
            }
        }
        report.global.sort_by_key(sort_key);
        report.scoped.sort_by_key(sort_key);
        Ok(report)
    }

    /// Lists the users who hold `perm` on the resource `(kind, id)`.
    ///
    /// A user counts if a direct or role grant covers `perm` (exact key, `*` or
    /// `<Category>::*`) and the grant is either global or scoped to exactly this
    /// resource. Reads the database, not the cache. The result is sorted and
    /// free of duplicates.
    pub fn users_with_access(
        &self,
        kind: ResourceKindKey,
        id: &str,
        perm: P,
    ) -> Result<Vec<String>, Error> {
        let key = perm.key();
        let category_wildcard = key
            .as_str()
            .split_once("::")
            .map(|(category, _)| format!("{category}::*"))
            .unwrap_or_else(|| "*".to_owned());
        let mut conn = self.inner.pool.get()?;
        let rows: Vec<UserIdRow> = sql_query(
            "SELECT DISTINCT g.user_id FROM ( \
                 SELECT user_id, permission_key, scope_kind, scope_id \
                   FROM air_tight.user_permissions \
                 UNION ALL \
                 SELECT ur.user_id, rp.permission_key, ur.scope_kind, ur.scope_id \
                   FROM air_tight.user_roles ur \
                   JOIN air_tight.role_permissions rp ON rp.role_id = ur.role_id \
             ) g \
             WHERE g.permission_key IN ('*', $1, $2) \
               AND (g.scope_kind IS NULL OR (g.scope_kind = $3 AND g.scope_id = $4)) \
             ORDER BY g.user_id",
        )
        .bind::<Text, _>(key.as_str())
        .bind::<Text, _>(category_wildcard)
        .bind::<Text, _>(kind.as_str())
        .bind::<Text, _>(id)
        .load(&mut conn)?;
        Ok(rows.into_iter().map(|r| r.user_id).collect())
    }

    /// Lists all roles with their permission keys, ordered by name.
    pub fn list_roles(&self) -> Result<Vec<RoleInfo>, Error> {
        let mut conn = self.inner.pool.get()?;
        let rows: Vec<(uuid::Uuid, String, Option<String>)> = roles::table
            .select((roles::id, roles::name, roles::description))
            .order(roles::name.asc())
            .load(&mut conn)?;
        let perms: Vec<(uuid::Uuid, String)> = role_permissions::table
            .select((role_permissions::role_id, role_permissions::permission_key))
            .order(role_permissions::permission_key.asc())
            .load(&mut conn)?;
        Ok(rows
            .into_iter()
            .map(|(id, name, description)| RoleInfo {
                id: RoleId::from_uuid(id),
                name,
                description,
                permissions: perms
                    .iter()
                    .filter(|(role, _)| *role == id)
                    .map(|(_, key)| PermissionKey::new(key.as_str()))
                    .collect(),
            })
            .collect())
    }

    /// Lists the permission catalog, ordered by key.
    pub fn list_permissions(&self) -> Result<Vec<PermissionInfo>, Error> {
        let mut conn = self.inner.pool.get()?;
        let rows: Vec<PermissionRow> = permissions::table
            .select((
                permissions::key,
                permissions::category,
                permissions::action,
                permissions::resource_kind,
                permissions::description,
            ))
            .order(permissions::key.asc())
            .load(&mut conn)?;
        Ok(rows
            .into_iter()
            .map(
                |(key, category, action, kind, description)| PermissionInfo {
                    key: PermissionKey::new(key),
                    category,
                    action,
                    resource_kind: kind.map(ResourceKindKey::new),
                    description,
                },
            )
            .collect())
    }
}
