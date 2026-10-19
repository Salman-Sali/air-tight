//! Database-backed tests for migrations, catalog sync and grant loading.

mod common;

use air_tight::schema::{audit_log, role_permissions, roles, user_permissions, user_roles};
use air_tight::{
    GrantSet, Interner, Permission, PermissionCategory, PermissionKey, ResourceKind,
    ResourceKindKey, RoleId, load_grant_set, migration_sql, run_migrations, sync_catalog,
};
use diesel::prelude::*;
use diesel::sql_types::{BigInt, Text};

#[derive(ResourceKind)]
pub enum DbResources {
    Workspace,
    Invoice,
}

#[derive(Permission)]
pub enum WorkspaceAction {
    Read,
    Update,
}

#[derive(Permission)]
pub enum BillingAction {
    Read,
    Manage,
}

#[derive(PermissionCategory)]
pub enum DbPerms {
    #[air_tight(resource = db_resources_kinds::Workspace)]
    Workspace(WorkspaceAction),
    Billing(BillingAction),
}

fn key(s: &str) -> PermissionKey {
    PermissionKey::new(s)
}

fn kind(s: &str) -> ResourceKindKey {
    ResourceKindKey::new(s)
}

fn grant(conn: &mut PgConnection, user: &str, permission: &str, scope: Option<(&str, &str)>) {
    diesel::insert_into(user_permissions::table)
        .values((
            user_permissions::user_id.eq(user),
            user_permissions::permission_key.eq(permission),
            user_permissions::scope_kind.eq(scope.map(|s| s.0)),
            user_permissions::scope_id.eq(scope.map(|s| s.1)),
        ))
        .execute(conn)
        .unwrap();
}

fn make_role(conn: &mut PgConnection, perms: &[&str]) -> RoleId {
    let id = RoleId::new();
    diesel::insert_into(roles::table)
        .values((
            roles::id.eq(id.as_uuid()),
            roles::name.eq(common::unique("role")),
        ))
        .execute(conn)
        .unwrap();
    for p in perms {
        diesel::insert_into(role_permissions::table)
            .values((
                role_permissions::role_id.eq(id.as_uuid()),
                role_permissions::permission_key.eq(*p),
            ))
            .execute(conn)
            .unwrap();
    }
    id
}

fn assign(conn: &mut PgConnection, user: &str, role: RoleId, scope: Option<(&str, &str)>) {
    diesel::insert_into(user_roles::table)
        .values((
            user_roles::user_id.eq(user),
            user_roles::role_id.eq(role.as_uuid()),
            user_roles::scope_kind.eq(scope.map(|s| s.0)),
            user_roles::scope_id.eq(scope.map(|s| s.1)),
        ))
        .execute(conn)
        .unwrap();
}

/// Loads a set after making sure the catalog keys are interned.
fn load(conn: &mut PgConnection, user: &str) -> GrantSet {
    sync_catalog::<DbPerms, DbResources>(conn).unwrap();
    load_grant_set(conn, user).unwrap()
}

#[derive(QueryableByName)]
struct Count {
    #[diesel(sql_type = BigInt)]
    n: i64,
}

#[test]
fn migrations_are_idempotent_and_recorded() {
    let Some(mut conn) = common::connect() else {
        return;
    };
    run_migrations(&mut conn).unwrap();
    run_migrations(&mut conn).unwrap();
    for (version, _) in migration_sql() {
        let c: Count =
            diesel::sql_query("SELECT count(*) AS n FROM air_tight.migrations WHERE version = $1")
                .bind::<Text, _>(*version)
                .get_result(&mut conn)
                .unwrap();
        assert_eq!(c.n, 1, "version {version} recorded exactly once");
    }
}

#[test]
fn migration_sql_content() {
    let all = migration_sql();
    assert_eq!(all[0].0, "0001_init");
    assert!(all[0].1.contains("CREATE TABLE air_tight.user_roles"));
    assert!(!all[0].1.contains("air_tight.migrations"));
    let versions: Vec<_> = all.iter().map(|m| m.0).collect();
    let mut sorted = versions.clone();
    sorted.sort();
    assert_eq!(versions, sorted);
}

#[test]
fn concurrent_migrations_are_safe() {
    if common::connect().is_none() {
        return;
    }
    let handles: Vec<_> = (0..4)
        .map(|_| {
            std::thread::spawn(|| {
                let mut conn = common::connect().unwrap();
                run_migrations(&mut conn).unwrap();
            })
        })
        .collect();
    for h in handles {
        h.join().unwrap();
    }
}

#[test]
fn sync_catalog_upserts_and_reports_stale() {
    let Some(mut conn) = common::connect() else {
        return;
    };
    let stale = common::unique("Stale::Key");
    diesel::sql_query(
        "INSERT INTO air_tight.permissions (key, category, action) VALUES ($1, 'Stale', 'Key')",
    )
    .bind::<Text, _>(&stale)
    .execute(&mut conn)
    .unwrap();
    let before: i64 = audit_log::table
        .filter(audit_log::action.eq("catalog_synced"))
        .count()
        .get_result(&mut conn)
        .unwrap();

    let report = sync_catalog::<DbPerms, DbResources>(&mut conn).unwrap();
    assert_eq!(report.permissions, 4);
    assert_eq!(report.resource_kinds, 2);
    assert!(report.stale_permissions.contains(&stale));
    assert!(
        !report
            .stale_permissions
            .iter()
            .any(|k| k == "Workspace::Read")
    );

    let (category, action, rk): (String, String, Option<String>) =
        air_tight::schema::permissions::table
            .find("Workspace::Update")
            .select((
                air_tight::schema::permissions::category,
                air_tight::schema::permissions::action,
                air_tight::schema::permissions::resource_kind,
            ))
            .first(&mut conn)
            .unwrap();
    assert_eq!(
        (category.as_str(), action.as_str()),
        ("Workspace", "Update")
    );
    assert_eq!(rk.as_deref(), Some("Workspace"));
    let billing_kind: Option<String> = air_tight::schema::permissions::table
        .find("Billing::Manage")
        .select(air_tight::schema::permissions::resource_kind)
        .first(&mut conn)
        .unwrap();
    assert_eq!(billing_kind, None);

    // Idempotent.
    sync_catalog::<DbPerms, DbResources>(&mut conn).unwrap();
    let after: i64 = audit_log::table
        .filter(audit_log::action.eq("catalog_synced"))
        .count()
        .get_result(&mut conn)
        .unwrap();
    assert!(after >= before + 2);
    let interner = Interner::global();
    assert!(interner.key_id("Billing::Read").is_some());
    assert!(interner.kind_id("Invoice").is_some());
}

#[test]
fn direct_global_grant() {
    let Some(mut conn) = common::connect() else {
        return;
    };
    let user = common::unique("user");
    grant(&mut conn, &user, "Billing::Read", None);
    let set = load(&mut conn, &user);
    assert!(set.allows(&key("Billing::Read"), None));
    assert!(!set.allows(&key("Billing::Manage"), None));
    // A global grant also satisfies scoped checks.
    assert!(set.allows(&key("Billing::Read"), Some((&kind("Workspace"), "w1"))));
}

#[test]
fn direct_scoped_grant() {
    let Some(mut conn) = common::connect() else {
        return;
    };
    let user = common::unique("user");
    grant(
        &mut conn,
        &user,
        "Workspace::Update",
        Some(("Workspace", "w1")),
    );
    let set = load(&mut conn, &user);
    let ws = kind("Workspace");
    assert!(set.allows(&key("Workspace::Update"), Some((&ws, "w1"))));
    assert!(!set.allows(&key("Workspace::Update"), Some((&ws, "w2"))));
    assert!(!set.allows(&key("Workspace::Update"), Some((&kind("Invoice"), "w1"))));
    assert!(!set.allows(&key("Workspace::Update"), None));
    assert!(!set.allows(&key("Workspace::Read"), Some((&ws, "w1"))));
}

#[test]
fn role_global_and_scoped() {
    let Some(mut conn) = common::connect() else {
        return;
    };
    let user = common::unique("user");
    let global_role = make_role(&mut conn, &["Billing::Manage"]);
    let scoped_role = make_role(&mut conn, &["Workspace::Read", "Workspace::Update"]);
    assign(&mut conn, &user, global_role, None);
    assign(&mut conn, &user, scoped_role, Some(("Workspace", "w7")));
    let set = load(&mut conn, &user);
    let ws = kind("Workspace");
    assert!(set.allows(&key("Billing::Manage"), None));
    assert!(!set.allows(&key("Workspace::Read"), None));
    assert!(set.allows(&key("Workspace::Read"), Some((&ws, "w7"))));
    assert!(set.allows(&key("Workspace::Update"), Some((&ws, "w7"))));
    assert!(!set.allows(&key("Workspace::Read"), Some((&ws, "w8"))));
}

#[test]
fn wildcards() {
    let Some(mut conn) = common::connect() else {
        return;
    };
    let ws = kind("Workspace");

    let category_user = common::unique("user");
    grant(&mut conn, &category_user, "Workspace::*", None);
    let set = load(&mut conn, &category_user);
    assert!(set.allows(&key("Workspace::Update"), None));
    assert!(!set.allows(&key("Billing::Read"), None));

    let scoped_user = common::unique("user");
    grant(
        &mut conn,
        &scoped_user,
        "Workspace::*",
        Some(("Workspace", "w1")),
    );
    let set = load(&mut conn, &scoped_user);
    assert!(set.allows(&key("Workspace::Read"), Some((&ws, "w1"))));
    assert!(!set.allows(&key("Workspace::Read"), Some((&ws, "w2"))));

    let all_user = common::unique("user");
    let role = make_role(&mut conn, &["*"]);
    assign(&mut conn, &all_user, role, None);
    let set = load(&mut conn, &all_user);
    assert!(set.allows(&key("Billing::Manage"), None));
    assert!(set.allows(&key("Workspace::Read"), Some((&ws, "anything"))));
}

#[test]
fn unknown_user_is_empty_and_unknown_keys_are_not_interned() {
    let Some(mut conn) = common::connect() else {
        return;
    };
    let set = load(&mut conn, &common::unique("nobody"));
    assert!(set.is_empty());
    assert_eq!(set.approx_bytes() as usize, std::mem::size_of::<GrantSet>());
    let never = common::unique("Never::Seen");
    assert!(!set.allows(&key(&never), None));
    assert!(Interner::global().key_id(&never).is_none());
}

#[test]
fn interner_ids_are_stable_and_lazy() {
    let interner = Interner::global();
    let name = common::unique("Lazy::Key");
    assert!(interner.key_id(&name).is_none());
    let a = interner.intern_key(&name).unwrap();
    assert_eq!(interner.intern_key(&name).unwrap(), a);
    assert_eq!(interner.key_id(&name), Some(a));
    let kind_name = common::unique("LazyKind");
    let k = interner.intern_kind(&kind_name).unwrap();
    assert_eq!(interner.kind_id(&kind_name), Some(k));
}

#[test]
fn approx_bytes_grows_with_content() {
    let Some(mut conn) = common::connect() else {
        return;
    };
    let user = common::unique("user");
    grant(&mut conn, &user, "Billing::Read", None);
    grant(
        &mut conn,
        &user,
        "Workspace::Read",
        Some(("Workspace", "w1")),
    );
    let set = load(&mut conn, &user);
    assert!(set.approx_bytes() as usize > std::mem::size_of::<GrantSet>());
}
