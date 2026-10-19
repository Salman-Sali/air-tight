//! Database-backed tests for the client: cache, check API, `Granted`.

mod common;

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use air_tight::schema::{role_permissions, roles, user_permissions, user_roles};
use air_tight::{
    AirTight, Condition, Error, GrantCache, GrantSet, MokaGrantCache, Permission,
    PermissionCategory, Principal, ResourceKind, ResourceKindKey, ResourceScope, RoleId,
};
use axum::extract::{FromRef, State};
use diesel::prelude::*;

#[derive(ResourceKind)]
pub enum ClResources {
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
pub enum ClPerms {
    #[air_tight(resource = cl_resources_kinds::Workspace)]
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

type Client = AirTight<ClPerms, User>;

const WS_READ: Condition<ClPerms> = Condition::Leaf(ClPerms::Workspace(WorkspaceAction::Read));
const WS_UPDATE: Condition<ClPerms> = Condition::Leaf(ClPerms::Workspace(WorkspaceAction::Update));
const BILL_READ: Condition<ClPerms> = Condition::Leaf(ClPerms::Billing(BillingAction::Read));
const BILL_MANAGE: Condition<ClPerms> = Condition::Leaf(ClPerms::Billing(BillingAction::Manage));

static AND: Condition<ClPerms> = Condition::All(&[WS_READ, BILL_READ]);
static OR: Condition<ClPerms> = Condition::Any(&[WS_UPDATE, BILL_MANAGE]);
const NOT: Condition<ClPerms> = Condition::Not(&BILL_MANAGE);
static READ_NOT_MANAGE: Condition<ClPerms> = Condition::All(&[WS_READ, NOT]);

/// Wraps the moka cache and counts inserts, i.e. database loads.
struct CountingCache {
    inner: MokaGrantCache,
    inserts: AtomicUsize,
}

impl CountingCache {
    fn new(ttl: Duration) -> Arc<Self> {
        Arc::new(CountingCache {
            inner: MokaGrantCache::new(ttl, 1 << 20),
            inserts: AtomicUsize::new(0),
        })
    }

    fn loads(&self) -> usize {
        self.inserts.load(Ordering::SeqCst)
    }
}

impl GrantCache for CountingCache {
    fn get(&self, user_id: &str) -> Option<Arc<GrantSet>> {
        self.inner.get(user_id)
    }
    fn insert(&self, user_id: String, set: Arc<GrantSet>) {
        self.inserts.fetch_add(1, Ordering::SeqCst);
        self.inner.insert(user_id, set);
    }
    fn invalidate(&self, user_id: &str) {
        self.inner.invalidate(user_id);
    }
    fn invalidate_all(&self) {
        self.inner.invalidate_all();
    }
}

fn client_with(cache: Arc<dyn GrantCache>) -> Option<Client> {
    let pool = common::pool()?;
    Some(
        Client::builder(pool)
            .resource_kinds::<ClResources>()
            .cache(cache)
            .build()
            .expect("build client"),
    )
}

fn client() -> Option<Client> {
    client_with(CountingCache::new(Duration::from_secs(60)))
}

fn conn() -> PgConnection {
    common::connect().expect("connected")
}

fn user() -> User {
    User(common::unique("user"))
}

fn workspace(id: &str) -> ResourceScope {
    ResourceScope::new(ResourceKindKey::new("Workspace"), id)
}

fn grant(user: &User, permission: &str, scope: Option<(&str, &str)>) {
    diesel::insert_into(user_permissions::table)
        .values((
            user_permissions::user_id.eq(&user.0),
            user_permissions::permission_key.eq(permission),
            user_permissions::scope_kind.eq(scope.map(|s| s.0)),
            user_permissions::scope_id.eq(scope.map(|s| s.1)),
        ))
        .execute(&mut conn())
        .unwrap();
}

fn assign_role(user: &User, perms: &[&str], scope: Option<(&str, &str)>) {
    let mut conn = conn();
    let id = RoleId::new();
    diesel::insert_into(roles::table)
        .values((
            roles::id.eq(id.as_uuid()),
            roles::name.eq(common::unique("role")),
        ))
        .execute(&mut conn)
        .unwrap();
    for p in perms {
        diesel::insert_into(role_permissions::table)
            .values((
                role_permissions::role_id.eq(id.as_uuid()),
                role_permissions::permission_key.eq(*p),
            ))
            .execute(&mut conn)
            .unwrap();
    }
    diesel::insert_into(user_roles::table)
        .values((
            user_roles::user_id.eq(&user.0),
            user_roles::role_id.eq(id.as_uuid()),
            user_roles::scope_kind.eq(scope.map(|s| s.0)),
            user_roles::scope_id.eq(scope.map(|s| s.1)),
        ))
        .execute(&mut conn)
        .unwrap();
}

fn missing(result: Result<impl std::fmt::Debug, Error>) -> Vec<String> {
    match result {
        Err(Error::Forbidden { missing }) => missing,
        other => panic!("expected Forbidden, got {other:?}"),
    }
}

#[test]
fn builder_requires_valid_channel() {
    let Some(pool) = common::pool() else {
        return;
    };
    let result = Client::builder(pool)
        .notify_channel("Bad-Name")
        .resource_kinds::<ClResources>()
        .build();
    assert!(matches!(result, Err(Error::Notify(_))));
}

#[test]
fn migration_sql_delegates() {
    assert_eq!(Client::migration_sql(), air_tight::migration_sql());
}

#[test]
fn sync_catalog_method() {
    let Some(client) = client() else {
        return;
    };
    let report = client.sync_catalog().unwrap();
    assert_eq!(report.permissions, ClPerms::all().len());
    assert_eq!(report.resource_kinds, 2);
}

#[test]
fn cache_miss_then_hit() {
    let cache = CountingCache::new(Duration::from_secs(60));
    let Some(client) = client_with(cache.clone()) else {
        return;
    };
    let u = user();
    grant(&u, "Billing::Read", None);
    let a = client.grant_set(&u.0).unwrap();
    let b = client.grant_set(&u.0).unwrap();
    assert!(Arc::ptr_eq(&a, &b));
    assert_eq!(cache.loads(), 1);
    client.check(&u.0, &BILL_READ, &[]).unwrap();
    client.authorize(&u, &BILL_READ, vec![]).unwrap();
    assert_eq!(cache.loads(), 1);
}

#[test]
fn invalidate_user_and_all_reload() {
    let cache = CountingCache::new(Duration::from_secs(60));
    let Some(client) = client_with(cache.clone()) else {
        return;
    };
    let (a, b) = (user(), user());
    grant(&a, "Billing::Read", None);
    grant(&b, "Billing::Read", None);
    client.check(&a.0, &BILL_READ, &[]).unwrap();
    client.check(&b.0, &BILL_READ, &[]).unwrap();
    grant(&a, "Billing::Manage", None);
    grant(&b, "Billing::Manage", None);
    assert!(client.check(&a.0, &BILL_MANAGE, &[]).is_err());

    client.invalidate_user(&a.0);
    client.check(&a.0, &BILL_MANAGE, &[]).unwrap();
    assert!(
        client.check(&b.0, &BILL_MANAGE, &[]).is_err(),
        "b still cached"
    );

    client.invalidate_all();
    client.check(&b.0, &BILL_MANAGE, &[]).unwrap();
    assert_eq!(cache.loads(), 4);
}

#[test]
fn idle_ttl_is_refreshed_by_access() {
    let cache = Arc::new(MokaGrantCache::new(Duration::from_millis(400), 1 << 20));
    let Some(client) = client_with(cache.clone()) else {
        return;
    };
    let u = user();
    grant(&u, "Billing::Read", None);
    let first = client.grant_set(&u.0).unwrap();
    for _ in 0..6 {
        std::thread::sleep(Duration::from_millis(120));
        let again = client.grant_set(&u.0).unwrap();
        assert!(
            Arc::ptr_eq(&first, &again),
            "access must keep the entry alive"
        );
    }
    std::thread::sleep(Duration::from_millis(900));
    cache.run_pending_tasks();
    assert!(cache.get(&u.0).is_none(), "idle entry must expire");
    let reloaded = client.grant_set(&u.0).unwrap();
    assert!(!Arc::ptr_eq(&first, &reloaded));
}

#[test]
fn scoped_versus_global_end_to_end() {
    let Some(client) = client() else {
        return;
    };
    let (a, b) = (common::unique("ws"), common::unique("ws"));
    let scoped = user();
    grant(&scoped, "Workspace::Read", Some(("Workspace", &a)));
    client.check(&scoped.0, &WS_READ, &[workspace(&a)]).unwrap();
    assert_eq!(
        missing(client.check(&scoped.0, &WS_READ, &[workspace(&b)])),
        ["Workspace::Read"]
    );
    assert!(
        client
            .check(&scoped.0, &WS_READ, &[workspace(&a), workspace(&b)])
            .is_err()
    );
    assert!(
        client.check(&scoped.0, &WS_READ, &[]).is_err(),
        "no scope means global check"
    );
    assert!(
        client
            .check(&scoped.0, &WS_UPDATE, &[workspace(&a)])
            .is_err()
    );

    let global = user();
    grant(&global, "Workspace::Read", None);
    client.check(&global.0, &WS_READ, &[]).unwrap();
    client
        .check(&global.0, &WS_READ, &[workspace(&a), workspace(&b)])
        .unwrap();

    let by_role = user();
    assign_role(&by_role, &["Workspace::Read"], Some(("Workspace", &a)));
    client
        .check(&by_role.0, &WS_READ, &[workspace(&a)])
        .unwrap();
    assert!(
        client
            .check(&by_role.0, &WS_READ, &[workspace(&b)])
            .is_err()
    );
}

#[test]
fn wildcards() {
    let Some(client) = client() else {
        return;
    };
    let id = common::unique("ws");
    let cat = user();
    grant(&cat, "Workspace::*", Some(("Workspace", &id)));
    client.check(&cat.0, &WS_UPDATE, &[workspace(&id)]).unwrap();
    assert!(client.check(&cat.0, &BILL_READ, &[]).is_err());

    let all = user();
    grant(&all, "*", None);
    client.check(&all.0, &BILL_MANAGE, &[]).unwrap();
    client.check(&all.0, &WS_UPDATE, &[workspace(&id)]).unwrap();
}

#[test]
fn boolean_conditions() {
    let Some(client) = client() else {
        return;
    };
    let u = user();
    grant(&u, "Workspace::Read", None);
    assert_eq!(missing(client.check(&u.0, &AND, &[])), ["Billing::Read"]);
    client.check(&u.0, &READ_NOT_MANAGE, &[]).unwrap();
    let mut all = missing(client.check(&u.0, &OR, &[]));
    all.sort();
    assert_eq!(all, ["Billing::Manage", "Workspace::Update"]);

    grant(&u, "Billing::Manage", None);
    client.invalidate_user(&u.0);
    client.check(&u.0, &OR, &[]).unwrap();
    assert!(client.check(&u.0, &READ_NOT_MANAGE, &[]).is_err());
    assert!(missing(client.check(&u.0, &READ_NOT_MANAGE, &[])).is_empty());
}

#[test]
fn granted_require_uses_snapshot() {
    let Some(client) = client() else {
        return;
    };
    let id = common::unique("ws");
    let u = user();
    grant(&u, "Workspace::Read", Some(("Workspace", &id)));
    let granted = client
        .authorize(&u, &WS_READ, vec![workspace(&id)])
        .unwrap();
    assert_eq!(granted.user_id(), u.0);
    assert_eq!(granted.scopes(), [workspace(&id)]);
    granted.require(&WS_READ).unwrap();
    assert_eq!(missing(granted.require(&WS_UPDATE)), ["Workspace::Update"]);

    // Later grants and invalidation do not change an existing snapshot.
    grant(&u, "Workspace::Update", Some(("Workspace", &id)));
    client.invalidate_user(&u.0);
    assert!(granted.require(&WS_UPDATE).is_err());
    let fresh = client
        .authorize(&u, &WS_UPDATE, vec![workspace(&id)])
        .unwrap();
    fresh.require(&WS_UPDATE).unwrap();
    let copy = fresh.clone();
    assert_eq!(copy.user_id(), fresh.user_id());
    assert!(format!("{copy:?}").contains("Granted"));

    assert!(client.authorize(&u, &BILL_READ, vec![]).is_err());
}

async fn async_flow(client: Client, cache: Arc<CountingCache>) {
    let u = user();
    grant(&u, "Billing::Read", None);
    let before = cache.loads();
    let granted = client
        .authorize_async(&u, &BILL_READ, vec![])
        .await
        .unwrap();
    assert_eq!(cache.loads(), before + 1, "miss loads once");
    granted.require(&BILL_READ).unwrap();
    client
        .authorize_async(&u, &BILL_READ, vec![])
        .await
        .unwrap();
    assert_eq!(cache.loads(), before + 1, "hit does not load");
    let denied = client.authorize_async(&u, &BILL_MANAGE, vec![]).await;
    assert_eq!(missing(denied), ["Billing::Manage"]);
}

fn assert_send<T: Send>(_: &T) {}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn authorize_async_multi_thread() {
    let cache = CountingCache::new(Duration::from_secs(60));
    let Some(client) = client_with(cache.clone()) else {
        return;
    };
    let fut = async_flow(client.clone(), cache.clone());
    assert_send(&fut);
    fut.await;
    // Also from a spawned task, proving the future is Send.
    tokio::spawn(async_flow(client, cache)).await.unwrap();
}

#[tokio::test(flavor = "current_thread")]
async fn authorize_async_current_thread() {
    let cache = CountingCache::new(Duration::from_secs(60));
    let Some(client) = client_with(cache.clone()) else {
        return;
    };
    async_flow(client, cache).await;
}

#[derive(Clone)]
struct AppState {
    air_tight: Client,
}

impl FromRef<AppState> for Client {
    fn from_ref(state: &AppState) -> Self {
        state.air_tight.clone()
    }
}

async fn handler(State(_client): State<Client>) -> &'static str {
    "ok"
}

#[test]
fn state_extraction_compiles_with_from_ref() {
    // Compiling this router is the assertion: the handler extracts
    // `State<AirTight<..>>` from an app state through `FromRef`.
    let _router: axum::Router<AppState> =
        axum::Router::new().route("/", axum::routing::get(handler));
    let Some(client) = client() else {
        return;
    };
    let state = AppState { air_tight: client };
    let _extracted = Client::from_ref(&state);
    let _router: axum::Router = axum::Router::new()
        .route("/", axum::routing::get(handler))
        .with_state(state);
}
