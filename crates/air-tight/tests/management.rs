//! Database-backed tests for the management API, audit log and invalidation.

mod common;

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use air_tight::schema::{audit_log, permissions, role_permissions, user_permissions};
use air_tight::{
    AirTight, Condition, Error, GrantCache, GrantSet, GrantSource, MokaGrantCache, Permission,
    PermissionCategory, PermissionKey, Principal, ResourceKind, ResourceKindKey, ResourceScope,
    RoleName,
};
use diesel::prelude::*;
use diesel::sql_query;
use diesel::sql_types::Text;

#[derive(ResourceKind)]
pub enum MgResources {
    Workspace,
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
pub enum MgPerms {
    #[air_tight(resource = mg_resources_kinds::Workspace)]
    Workspace(WorkspaceAction),
    Billing(BillingAction),
}

#[derive(Clone)]
struct User(String);

impl Principal for User {
    fn air_tight_id(&self) -> String {
        self.0.clone()
    }
}

type Client = AirTight<MgPerms, User>;

const WS_READ: Condition<MgPerms> = Condition::Leaf(MgPerms::Workspace(WorkspaceAction::Read));
const BILL_READ: Condition<MgPerms> = Condition::Leaf(MgPerms::Billing(BillingAction::Read));
const BILL_MANAGE: Condition<MgPerms> = Condition::Leaf(MgPerms::Billing(BillingAction::Manage));

/// A channel no other test uses, so `all` messages cannot cross tests.
fn channel() -> String {
    format!("t_{}", uuid::Uuid::new_v4().simple())
}

fn client_on(channel: &str) -> Option<Client> {
    client_with(
        channel,
        Arc::new(MokaGrantCache::new(Duration::from_secs(60), 1 << 20)),
    )
}

fn client_with(channel: &str, cache: Arc<dyn GrantCache>) -> Option<Client> {
    let pool = common::pool()?;
    Some(
        Client::builder(pool)
            .resource_kinds::<MgResources>()
            .notify_channel(channel)
            .cache(cache)
            .build()
            .expect("build client"),
    )
}

fn client() -> Option<Client> {
    client_on(&channel())
}

fn conn() -> PgConnection {
    common::connect().expect("connected")
}

fn key(s: &str) -> PermissionKey {
    PermissionKey::new(s)
}

fn workspace(id: &str) -> ResourceScope {
    ResourceScope::new(ResourceKindKey::new("Workspace"), id)
}

fn role_name() -> RoleName {
    RoleName::new(&common::unique("role")).unwrap()
}

fn allowed(
    client: &Client,
    user: &str,
    cond: &Condition<MgPerms>,
    scopes: &[ResourceScope],
) -> bool {
    match client.check(user, cond, scopes) {
        Ok(()) => true,
        Err(Error::Forbidden { .. }) => false,
        Err(other) => panic!("unexpected error: {other:?}"),
    }
}

/// Polls `f` for up to five seconds.
fn eventually(what: &str, mut f: impl FnMut() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        if f() {
            return;
        }
        std::thread::sleep(Duration::from_millis(25));
    }
    panic!("timed out waiting for: {what}");
}

#[derive(Queryable, Debug)]
struct Audit {
    actor: Option<String>,
    action: String,
    target: Option<String>,
    permission: Option<String>,
    scope_kind: Option<String>,
    scope_id: Option<String>,
}

fn audit_for_user(user: &str) -> Vec<Audit> {
    audit_log::table
        .filter(audit_log::target_user_id.eq(user))
        .order(audit_log::id.asc())
        .select((
            audit_log::actor_user_id,
            audit_log::action,
            audit_log::target_user_id,
            audit_log::permission_key,
            audit_log::scope_kind,
            audit_log::scope_id,
        ))
        .load(&mut conn())
        .unwrap()
}

fn audit_actions_for_role(role: air_tight::RoleId) -> Vec<(Option<String>, String)> {
    audit_log::table
        .filter(audit_log::role_id.eq(role.as_uuid()))
        .order(audit_log::id.asc())
        .select((audit_log::actor_user_id, audit_log::action))
        .load(&mut conn())
        .unwrap()
}

#[test]
fn grant_and_revoke_update_report_cache_and_audit() {
    let Some(client) = client() else {
        return;
    };
    let u = common::unique("user");
    assert!(!allowed(&client, &u, &BILL_READ, &[]));

    client
        .grant(Some("admin-1"), &u, key("Billing::Read"), None)
        .unwrap();
    assert!(allowed(&client, &u, &BILL_READ, &[]));
    // Idempotent: the second grant is a no-op, with no second audit row.
    client
        .grant(Some("admin-1"), &u, key("Billing::Read"), None)
        .unwrap();
    client
        .grant(None, &u, key("Workspace::Update"), Some(workspace("w1")))
        .unwrap();

    let report = client.effective_grants(&u).unwrap();
    assert_eq!(report.global.len(), 1);
    assert_eq!(report.global[0].key, key("Billing::Read"));
    assert_eq!(report.global[0].source, GrantSource::Direct);
    assert_eq!(report.scoped.len(), 1);
    assert_eq!(report.scoped[0].scope, Some(workspace("w1")));

    client
        .revoke(Some("admin-2"), &u, key("Billing::Read"), None)
        .unwrap();
    assert!(!allowed(&client, &u, &BILL_READ, &[]));
    // Revoking something absent is Ok and not audited.
    client
        .revoke(Some("admin-2"), &u, key("Billing::Read"), None)
        .unwrap();
    client
        .revoke(None, &u, key("Workspace::Update"), Some(workspace("w1")))
        .unwrap();
    let report = client.effective_grants(&u).unwrap();
    assert!(report.global.is_empty() && report.scoped.is_empty());

    let audit = audit_for_user(&u);
    let actions: Vec<(&str, Option<&str>)> = audit
        .iter()
        .map(|a| (a.action.as_str(), a.actor.as_deref()))
        .collect();
    assert_eq!(
        actions,
        vec![
            ("grant_added", Some("admin-1")),
            ("grant_added", None),
            ("grant_revoked", Some("admin-2")),
            ("grant_revoked", None),
        ]
    );
    assert_eq!(audit[0].permission.as_deref(), Some("Billing::Read"));
    assert_eq!(audit[0].target.as_deref(), Some(u.as_str()));
    assert_eq!(audit[1].scope_kind.as_deref(), Some("Workspace"));
    assert_eq!(audit[1].scope_id.as_deref(), Some("w1"));
}

#[test]
fn grant_validates_keys_and_accepts_wildcards() {
    let Some(client) = client() else {
        return;
    };
    let u = common::unique("user");
    let err = client
        .grant(None, &u, key("Nope::Nothing"), None)
        .unwrap_err();
    assert!(
        matches!(&err, Error::Catalog(m) if m.contains("Nope::Nothing")),
        "{err:?}"
    );
    client.grant(None, &u, key("Billing::*"), None).unwrap();
    assert!(allowed(&client, &u, &BILL_MANAGE, &[]));
    assert!(audit_for_user(&u).len() == 1);
}

#[test]
fn role_lifecycle_with_scoped_assignment() {
    let Some(client) = client() else {
        return;
    };
    let u = common::unique("user");
    let name = role_name();
    let role = client
        .create_role(Some("admin"), &name, Some("a test role"))
        .unwrap();
    client
        .set_role_permissions(Some("admin"), role, &[key("Workspace::Read")])
        .unwrap();
    client
        .assign_role(Some("admin"), &u, role, Some(workspace("w1")))
        .unwrap();
    // Idempotent for scoped and global rows alike.
    client
        .assign_role(Some("admin"), &u, role, Some(workspace("w1")))
        .unwrap();

    assert!(allowed(&client, &u, &WS_READ, &[workspace("w1")]));
    assert!(!allowed(&client, &u, &WS_READ, &[workspace("w2")]));
    assert!(!allowed(&client, &u, &WS_READ, &[]));

    let report = client.effective_grants(&u).unwrap();
    assert_eq!(report.scoped.len(), 1);
    assert!(report.global.is_empty());
    match &report.scoped[0].source {
        GrantSource::Role { id, name: n } => {
            assert_eq!(*id, role);
            assert_eq!(n, name.as_str());
        }
        other => panic!("{other:?}"),
    }

    // Replacing the set invalidates holders: Read is gone, Billing::Read is in.
    client
        .set_role_permissions(None, role, &[key("Billing::Read")])
        .unwrap();
    assert!(!allowed(&client, &u, &WS_READ, &[workspace("w1")]));
    assert!(!allowed(&client, &u, &BILL_READ, &[]));
    client.assign_role(None, &u, role, None).unwrap();
    assert!(allowed(&client, &u, &BILL_READ, &[]));

    client.revoke_role(Some("admin"), &u, role, None).unwrap();
    assert!(!allowed(&client, &u, &BILL_READ, &[]));
    client.revoke_role(None, &u, role, None).unwrap();
    client
        .revoke_role(None, &u, role, Some(workspace("w1")))
        .unwrap();
    assert!(client.effective_grants(&u).unwrap().scoped.is_empty());

    let actions: Vec<String> = audit_for_user(&u).into_iter().map(|a| a.action).collect();
    assert_eq!(
        actions,
        vec![
            "role_assigned",
            "role_assigned",
            "role_revoked",
            "role_revoked"
        ]
    );
    assert_eq!(
        audit_actions_for_role(role)[..3],
        [
            (Some("admin".to_owned()), "role_created".to_owned()),
            (Some("admin".to_owned()), "role_permissions_set".to_owned()),
            (Some("admin".to_owned()), "role_assigned".to_owned()),
        ]
    );
}

#[test]
fn set_role_permissions_validates_keys() {
    let Some(client) = client() else {
        return;
    };
    let role = client.create_role(None, &role_name(), None).unwrap();
    client
        .set_role_permissions(None, role, &[key("Billing::Read")])
        .unwrap();
    let err = client
        .set_role_permissions(None, role, &[key("Billing::Manage"), key("Bogus::Key")])
        .unwrap_err();
    assert!(
        matches!(&err, Error::Catalog(m) if m.contains("Bogus::Key")),
        "{err:?}"
    );
    // The failed call changed nothing.
    let info = client
        .list_roles()
        .unwrap()
        .into_iter()
        .find(|r| r.id == role)
        .unwrap();
    assert_eq!(info.permissions, vec![key("Billing::Read")]);

    for bad in ["::*", "A::B::*", ""] {
        assert!(
            client
                .set_role_permissions(None, role, &[key(bad)])
                .is_err(),
            "{bad}"
        );
    }
    client
        .set_role_permissions(None, role, &[key("*"), key("Workspace::*")])
        .unwrap();
    let info = client
        .list_roles()
        .unwrap()
        .into_iter()
        .find(|r| r.id == role)
        .unwrap();
    assert_eq!(info.permissions, vec![key("*"), key("Workspace::*")]);
    client.set_role_permissions(None, role, &[]).unwrap();
    let info = client
        .list_roles()
        .unwrap()
        .into_iter()
        .find(|r| r.id == role)
        .unwrap();
    assert!(info.permissions.is_empty());
}

#[test]
fn delete_role_invalidates_affected_users() {
    let Some(client) = client() else {
        return;
    };
    let (a, b) = (common::unique("user"), common::unique("user"));
    let role = client.create_role(None, &role_name(), None).unwrap();
    client
        .set_role_permissions(None, role, &[key("Workspace::Read")])
        .unwrap();
    client.assign_role(None, &a, role, None).unwrap();
    client
        .assign_role(None, &b, role, Some(workspace("w")))
        .unwrap();
    assert!(allowed(&client, &a, &WS_READ, &[workspace("w")]));
    assert!(allowed(&client, &b, &WS_READ, &[workspace("w")]));

    client.delete_role(Some("admin"), role).unwrap();
    assert!(!allowed(&client, &a, &WS_READ, &[workspace("w")]));
    assert!(!allowed(&client, &b, &WS_READ, &[workspace("w")]));
    assert_eq!(
        audit_actions_for_role(role).last().unwrap(),
        &(Some("admin".to_owned()), "role_deleted".to_owned())
    );
    // Deleting again is an Ok no-op.
    client.delete_role(None, role).unwrap();
    assert!(client.list_roles().unwrap().iter().all(|r| r.id != role));
}

#[test]
fn invalidate_role_is_precise() {
    let Some(client) = client() else {
        return;
    };
    let (holder, other) = (common::unique("user"), common::unique("user"));
    let role = client.create_role(None, &role_name(), None).unwrap();
    client
        .set_role_permissions(None, role, &[key("Billing::Read")])
        .unwrap();
    client.assign_role(None, &holder, role, None).unwrap();
    let before_holder = client.grant_set(&holder).unwrap();
    let before_other = client.grant_set(&other).unwrap();
    client.invalidate_role(role).unwrap();
    assert!(!Arc::ptr_eq(
        &before_holder,
        &client.grant_set(&holder).unwrap()
    ));
    assert!(Arc::ptr_eq(
        &before_other,
        &client.grant_set(&other).unwrap()
    ));
}

#[test]
fn users_with_access_covers_roles_scopes_and_wildcards() {
    let Some(client) = client() else {
        return;
    };
    let tag = common::unique("ws");
    let [
        global,
        scoped,
        other_scope,
        wild_all,
        wild_cat,
        via_role,
        via_scoped_role,
        nobody,
    ] = std::array::from_fn(|i| format!("{}-{i}", common::unique("user")));
    let ws = ResourceKindKey::new("Workspace");
    let scope = |id: &str| Some(ResourceScope::new(ws.clone(), id));

    client
        .grant(None, &global, key("Workspace::Read"), None)
        .unwrap();
    client
        .grant(None, &scoped, key("Workspace::Read"), scope(&tag))
        .unwrap();
    client
        .grant(
            None,
            &other_scope,
            key("Workspace::Read"),
            scope("elsewhere"),
        )
        .unwrap();
    client.grant(None, &wild_all, key("*"), None).unwrap();
    client
        .grant(None, &wild_cat, key("Workspace::*"), scope(&tag))
        .unwrap();
    client
        .grant(None, &nobody, key("Workspace::Update"), None)
        .unwrap();
    client
        .grant(None, &nobody, key("Billing::*"), None)
        .unwrap();
    let role = client.create_role(None, &role_name(), None).unwrap();
    client
        .set_role_permissions(None, role, &[key("Workspace::Read")])
        .unwrap();
    client.assign_role(None, &via_role, role, None).unwrap();
    client
        .assign_role(None, &via_scoped_role, role, scope(&tag))
        .unwrap();
    // Duplicate sources must not duplicate the user.
    client
        .grant(None, &via_role, key("Workspace::Read"), None)
        .unwrap();

    let found = client
        .users_with_access(ws.clone(), &tag, MgPerms::Workspace(WorkspaceAction::Read))
        .unwrap();
    let mine: Vec<String> = found
        .into_iter()
        .filter(|u| {
            [
                &global,
                &scoped,
                &other_scope,
                &wild_all,
                &wild_cat,
                &via_role,
                &via_scoped_role,
                &nobody,
            ]
            .contains(&u)
        })
        .collect();
    let mut expected = vec![
        global,
        scoped,
        wild_all,
        wild_cat,
        via_role,
        via_scoped_role,
    ];
    expected.sort();
    assert_eq!(mine, expected);
}

#[test]
fn rename_permission_rewrites_and_merges() {
    let Some(client) = client() else {
        return;
    };
    let tag = uuid::Uuid::new_v4().simple().to_string();
    let (old, new, taken) = (
        format!("Old{tag}::Do"),
        format!("New{tag}::Do"),
        format!("Taken{tag}::Do"),
    );
    let mut c = conn();
    for k in [&old, &taken] {
        let (cat, act) = k.split_once("::").unwrap();
        diesel::insert_into(permissions::table)
            .values((
                permissions::key.eq(k),
                permissions::category.eq(cat),
                permissions::action.eq(act),
            ))
            .execute(&mut c)
            .unwrap();
    }
    let (u1, u2) = (common::unique("user"), common::unique("user"));
    let role = client.create_role(None, &role_name(), None).unwrap();
    client
        .set_role_permissions(None, role, &[key(&old), key("Billing::Read")])
        .unwrap();
    client.grant(None, &u1, key(&old), None).unwrap();
    client
        .grant(None, &u2, key(&old), Some(workspace("w")))
        .unwrap();
    client
        .grant(None, &u2, key(&taken), Some(workspace("w")))
        .unwrap();
    client.assign_role(None, &u1, role, None).unwrap();
    let _ = client.grant_set(&u1).unwrap();

    // Plain rename: the new key does not exist anywhere yet.
    client
        .rename_permission(Some("admin"), &key(&old), &key(&new))
        .unwrap();
    let keys: Vec<String> = client
        .effective_grants(&u1)
        .unwrap()
        .global
        .into_iter()
        .map(|e| e.key.to_string())
        .collect();
    assert!(keys.contains(&new) && !keys.contains(&old), "{keys:?}");
    let listed: Vec<String> = client
        .list_permissions()
        .unwrap()
        .into_iter()
        .map(|p| p.key.to_string())
        .collect();
    assert!(listed.contains(&new) && !listed.contains(&old));
    let info = client
        .list_permissions()
        .unwrap()
        .into_iter()
        .find(|p| p.key == key(&new))
        .unwrap();
    assert_eq!(info.category, format!("New{tag}"));
    assert_eq!(info.action, "Do");

    // Merge: u2 holds both `new` (renamed from old) and `taken` on the same
    // scope; renaming taken -> new must drop the duplicate, not fail.
    client
        .rename_permission(None, &key(&taken), &key(&new))
        .unwrap();
    let u2_keys: Vec<String> = user_permissions::table
        .filter(user_permissions::user_id.eq(&u2))
        .select(user_permissions::permission_key)
        .load(&mut c)
        .unwrap();
    assert_eq!(u2_keys, vec![new.clone()]);
    let role_keys: Vec<String> = role_permissions::table
        .filter(role_permissions::role_id.eq(role.as_uuid()))
        .select(role_permissions::permission_key)
        .order(role_permissions::permission_key.asc())
        .load(&mut c)
        .unwrap();
    assert_eq!(role_keys, vec!["Billing::Read".to_owned(), new.clone()]);
    let taken_rows: i64 = permissions::table
        .filter(permissions::key.eq(&taken))
        .count()
        .get_result(&mut c)
        .unwrap();
    assert_eq!(taken_rows, 0);

    let audited: Vec<(Option<String>, String)> = audit_log::table
        .filter(audit_log::permission_key.eq(&old))
        .filter(audit_log::action.eq("permission_renamed"))
        .select((audit_log::actor_user_id, audit_log::action))
        .load(&mut c)
        .unwrap();
    assert_eq!(
        audited,
        vec![(Some("admin".to_owned()), "permission_renamed".to_owned())]
    );
    assert!(
        client
            .rename_permission(None, &key(&new), &key(&new))
            .is_err()
    );
    assert!(
        client
            .rename_permission(None, &key(&new), &key("nocolons"))
            .is_err()
    );
}

#[test]
fn list_roles_and_permissions() {
    let Some(client) = client() else {
        return;
    };
    let name = role_name();
    let role = client.create_role(None, &name, Some("desc")).unwrap();
    client
        .set_role_permissions(None, role, &[key("Workspace::Update"), key("Billing::*")])
        .unwrap();
    let info = client
        .list_roles()
        .unwrap()
        .into_iter()
        .find(|r| r.id == role)
        .unwrap();
    assert_eq!(info.name, name.as_str());
    assert_eq!(info.description.as_deref(), Some("desc"));
    assert_eq!(
        info.permissions,
        vec![key("Billing::*"), key("Workspace::Update")]
    );
    assert!(
        client.create_role(None, &name, None).is_err(),
        "names are unique"
    );
    assert!(RoleName::new("  ").is_err());

    let perms = client.list_permissions().unwrap();
    let update = perms
        .iter()
        .find(|p| p.key == key("Workspace::Update"))
        .unwrap();
    assert_eq!(update.category, "Workspace");
    assert_eq!(update.action, "Update");
    assert_eq!(
        update.resource_kind,
        Some(ResourceKindKey::new("Workspace"))
    );
    let billing = perms
        .iter()
        .find(|p| p.key == key("Billing::Read"))
        .unwrap();
    assert_eq!(billing.resource_kind, None);
}

#[test]
fn notify_reaches_other_clients_on_the_same_channel() {
    let chan = channel();
    let (Some(a), Some(b)) = (client_on(&chan), client_on(&chan)) else {
        return;
    };
    let u = common::unique("user");
    assert!(!allowed(&a, &u, &BILL_READ, &[]));

    // B mutates through the management API: user payload.
    b.grant(None, &u, key("Billing::Read"), None).unwrap();
    eventually("A sees B's grant", || allowed(&a, &u, &BILL_READ, &[]));

    // Raw SQL plus pg_notify, as an external writer would do it.
    let u2 = common::unique("user");
    assert!(!allowed(&a, &u2, &BILL_MANAGE, &[]));
    let mut c = conn();
    diesel::insert_into(user_permissions::table)
        .values((
            user_permissions::user_id.eq(&u2),
            user_permissions::permission_key.eq("Billing::Manage"),
        ))
        .execute(&mut c)
        .unwrap();
    sql_query("SELECT pg_notify($1, $2)")
        .bind::<Text, _>(&chan)
        .bind::<Text, _>(format!("user:{u2}"))
        .execute(&mut c)
        .unwrap();
    eventually("A sees the raw grant", || {
        allowed(&a, &u2, &BILL_MANAGE, &[])
    });

    // Role payload.
    let u3 = common::unique("user");
    let role = b.create_role(None, &role_name(), None).unwrap();
    b.assign_role(None, &u3, role, None).unwrap();
    assert!(!allowed(&a, &u3, &BILL_READ, &[]));
    b.set_role_permissions(None, role, &[key("Billing::Read")])
        .unwrap();
    eventually("A sees the role change", || {
        allowed(&a, &u3, &BILL_READ, &[])
    });

    // Unknown payloads are ignored; `all` drops everything.
    sql_query("SELECT pg_notify($1, 'garbage')")
        .bind::<Text, _>(&chan)
        .execute(&mut c)
        .unwrap();
    let warm = a.grant_set(&u).unwrap();
    sql_query("SELECT pg_notify($1, 'all')")
        .bind::<Text, _>(&chan)
        .execute(&mut c)
        .unwrap();
    eventually("A drops everything on `all`", || {
        !Arc::ptr_eq(&warm, &a.grant_set(&u).unwrap())
    });
}

#[test]
fn listener_reconnects_after_the_connection_dies() {
    let chan = channel();
    let (Some(a), Some(b)) = (client_on(&chan), client_on(&chan)) else {
        return;
    };
    let u = common::unique("user");
    assert!(!allowed(&a, &u, &BILL_READ, &[]));
    let mut c = conn();
    let killed: Vec<i32> = sql_query(
        "SELECT pg_terminate_backend(pid)::int AS n FROM pg_stat_activity \
         WHERE query = $1 AND pid <> pg_backend_pid()",
    )
    .bind::<Text, _>(format!("LISTEN {chan}"))
    .load::<Killed>(&mut c)
    .unwrap()
    .into_iter()
    .map(|k| k.n)
    .collect();
    assert_eq!(killed.len(), 2, "one listener per client");
    // Both listeners come back (after their backoff) and still deliver.
    std::thread::sleep(Duration::from_millis(500));
    b.grant(None, &u, key("Billing::Read"), None).unwrap();
    eventually("A sees the grant after reconnecting", || {
        allowed(&a, &u, &BILL_READ, &[])
    });
}

#[derive(QueryableByName)]
struct Killed {
    #[diesel(sql_type = diesel::sql_types::Integer)]
    n: i32,
}

/// A cache that runs a hook just before every insert, to model an
/// invalidation that races with a grant-set load.
struct RacyCache {
    inner: MokaGrantCache,
    before_insert: Mutex<Option<Box<dyn Fn() + Send + Sync>>>,
}

impl GrantCache for RacyCache {
    fn get(&self, user_id: &str) -> Option<Arc<GrantSet>> {
        self.inner.get(user_id)
    }
    fn insert(&self, user_id: String, set: Arc<GrantSet>) {
        if let Some(hook) = self.before_insert.lock().unwrap().as_ref() {
            hook();
        }
        self.inner.insert(user_id, set);
    }
    fn invalidate(&self, user_id: &str) {
        self.inner.invalidate(user_id);
    }
    fn invalidate_all(&self) {
        self.inner.invalidate_all();
    }
}

#[test]
fn stale_load_is_not_left_in_the_cache() {
    let cache = Arc::new(RacyCache {
        inner: MokaGrantCache::new(Duration::from_secs(60), 1 << 20),
        before_insert: Mutex::new(None),
    });
    let Some(client) = client_with(&channel(), cache.clone()) else {
        return;
    };
    let u = common::unique("user");
    client.grant(None, &u, key("Billing::Read"), None).unwrap();

    for invalidate_everything in [false, true] {
        let hook_client = client.clone();
        let hook_user = u.clone();
        *cache.before_insert.lock().unwrap() = Some(Box::new(move || {
            if invalidate_everything {
                hook_client.invalidate_all();
            } else {
                hook_client.invalidate_user(&hook_user);
            }
        }));
        // The caller still gets the freshly loaded set...
        let set = client.grant_set(&u).unwrap();
        assert!(set.approx_bytes() > 0);
        // ...but the set that raced with an invalidation was not kept.
        assert!(cache.get(&u).is_none(), "stale set stayed cached");
        cache.inner.invalidate_all();
    }

    // Without a race the load is cached as usual.
    *cache.before_insert.lock().unwrap() = None;
    client.grant_set(&u).unwrap();
    assert!(cache.get(&u).is_some());
}
