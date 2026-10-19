use diesel::prelude::*;
use diesel::sql_query;
use serde_json::json;

use crate::schema::{audit_log, permissions, resource_kinds};
use crate::{CatalogSyncReport, Error, Interner};
use air_tight_core::{PermissionCategory, ResourceKind};

/// Fixed key of the advisory lock serialising catalog syncs.
const SYNC_LOCK_KEY: i64 = 0x41_49_52_54_43_41_54_31;

#[derive(Insertable)]
#[diesel(table_name = permissions)]
struct NewPermission<'a> {
    key: &'a str,
    category: &'a str,
    action: &'a str,
    resource_kind: Option<&'a str>,
}

#[derive(Insertable)]
#[diesel(table_name = resource_kinds)]
struct NewResourceKind<'a> {
    key: &'a str,
}

#[derive(Insertable)]
#[diesel(table_name = audit_log)]
struct NewAuditRow<'a> {
    action: &'a str,
    detail: Option<serde_json::Value>,
}

/// Upserts the code catalog into the database and interns its keys.
///
/// In one transaction this upserts every `P::all()` permission and every
/// `R::all()` resource kind, warns about database permission keys that code no
/// longer defines, and writes one `catalog_synced` audit row.
pub fn sync_catalog<P: PermissionCategory, R: ResourceKind>(
    conn: &mut PgConnection,
) -> Result<CatalogSyncReport, Error> {
    let mut perms: Vec<(String, &'static str, String, Option<String>)> = Vec::new();
    for p in P::all() {
        let key = p.key().as_str().to_owned();
        let action = key
            .split_once("::")
            .map(|(_, a)| a.to_owned())
            .ok_or_else(|| Error::Catalog(format!("permission key without `::`: {key}")))?;
        let kind = p.resource_kind().map(|k| k.as_str().to_owned());
        perms.push((key, p.category_name(), action, kind));
    }
    perms.sort();
    let mut kinds: Vec<String> = R::all()
        .iter()
        .map(|k| k.key().as_str().to_owned())
        .collect();
    kinds.sort();

    let stale = conn.transaction::<Vec<String>, diesel::result::Error, _>(|conn| {
        sql_query("SELECT pg_advisory_xact_lock($1)")
            .bind::<diesel::sql_types::BigInt, _>(SYNC_LOCK_KEY)
            .execute(conn)?;
        for (key, category, action, kind) in &perms {
            let row = NewPermission {
                key,
                category,
                action,
                resource_kind: kind.as_deref(),
            };
            diesel::insert_into(permissions::table)
                .values(&row)
                .on_conflict(permissions::key)
                .do_update()
                .set((
                    permissions::category.eq(category),
                    permissions::action.eq(action),
                    permissions::resource_kind.eq(kind.as_deref()),
                ))
                .execute(conn)?;
        }
        for kind in &kinds {
            diesel::insert_into(resource_kinds::table)
                .values(&NewResourceKind { key: kind })
                .on_conflict_do_nothing()
                .execute(conn)?;
        }
        let in_code: Vec<&str> = perms.iter().map(|p| p.0.as_str()).collect();
        let stale: Vec<String> = permissions::table
            .filter(permissions::key.ne_all(in_code))
            .select(permissions::key)
            .order(permissions::key.asc())
            .load(conn)?;
        diesel::insert_into(audit_log::table)
            .values(&NewAuditRow {
                action: "catalog_synced",
                detail: Some(json!({
                    "permissions": perms.len(),
                    "resource_kinds": kinds.len(),
                    "stale_permissions": stale,
                })),
            })
            .execute(conn)?;
        Ok(stale)
    })?;

    for key in &stale {
        tracing::warn!(permission = %key, "permission key in the database is absent from code");
    }
    let interner = Interner::global();
    for (key, ..) in &perms {
        interner.intern_key(key)?;
    }
    for kind in &kinds {
        interner.intern_kind(kind)?;
    }
    Ok(CatalogSyncReport {
        permissions: perms.len(),
        resource_kinds: kinds.len(),
        stale_permissions: stale,
    })
}
