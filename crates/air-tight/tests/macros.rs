//! End-to-end tests of `#[permission]`, `#[unrestricted]`, `setup!` and the
//! typed router. Tests needing a database skip when
//! `AIR_TIGHT_TEST_DATABASE_URL` is unset.

mod common;

use air_tight::{
    AirTight, Error, IdentifyResources, Permission, PermissionCategory, PermissionKey, Principal,
    ResourceKind, ResourceKindKey, ResourceScope, Router, registry, routing,
};
use axum::body::Body;
use axum::http::{Request, StatusCode};
use tower::ServiceExt;

#[derive(ResourceKind)]
pub enum Kinds {
    Workspace,
    Invoice,
}

#[derive(Permission)]
pub enum WsAction {
    Read,
    Update,
}

#[derive(Permission)]
pub enum InvAction {
    Read,
    Create,
}

#[derive(Permission)]
pub enum BillAction {
    Read,
    Manage,
}

#[derive(PermissionCategory)]
pub enum Perms {
    #[air_tight(resource = kinds_kinds::Workspace)]
    Workspace(WsAction),
    #[air_tight(resource = kinds_kinds::Invoice)]
    Invoice(InvAction),
    Billing(BillAction),
}

#[derive(Clone, Debug)]
pub struct User {
    id: String,
}

impl Principal for User {
    fn air_tight_id(&self) -> String {
        self.id.clone()
    }
}

air_tight::setup!(perms = Perms, user = User, resources = Kinds);

type Client = AirTight<Perms, User>;

mod handlers {
    use air_tight::{Condition, CurrentUser, Error, Granted, IdentifyResources};
    use axum::Json;
    use axum::extract::State;
    use axum::response::IntoResponse;
    use serde::Deserialize;

    use super::kinds_kinds::{Invoice, Workspace};
    use super::{BillAction, Perms};

    #[derive(Deserialize, IdentifyResources)]
    pub struct WsBody {
        #[air_tight(resource = Workspace)]
        pub workspace: String,
        #[air_tight(resource = Invoice)]
        pub invoices: Vec<String>,
    }

    /// Health check.
    #[air_tight::unrestricted]
    pub async fn health() -> &'static str {
        "ok"
    }

    #[air_tight::permission(Workspace(Read), resources = [body])]
    pub async fn read_ws(Json(body): Json<WsBody>) -> Result<String, Error> {
        Ok(format!("read {}", body.workspace))
    }

    #[air_tight::permission(Workspace(Update) && Invoice(Create), resources = [body])]
    pub async fn create_invoices(Json(body): Json<WsBody>) -> Result<String, Error> {
        Ok(format!("{} invoices", body.invoices.len()))
    }

    #[air_tight::permission(Billing(Read))]
    pub async fn billing() -> impl IntoResponse {
        "billing"
    }

    #[air_tight::permission(any(Billing(Manage), Billing(Read) && !Workspace(Update)))]
    pub async fn odd() -> String {
        "odd".to_owned()
    }

    #[air_tight::permission(Billing(Read && Manage))]
    pub async fn both_billing() -> &'static str {
        "both"
    }

    static NEEDS_MANAGE: Condition<Perms> = Condition::Leaf(Perms::Billing(BillAction::Manage));

    fn manage_or_read(granted: &Granted<Perms>) -> &'static str {
        if granted.require(&NEEDS_MANAGE).is_ok() {
            "manager"
        } else {
            "reader"
        }
    }

    #[air_tight::permission(Billing(Read))]
    pub async fn who(n: axum::extract::Query<std::collections::HashMap<String, String>>) -> String {
        let _ = n;
        manage_or_read(&granted).to_owned()
    }

    #[air_tight::permission(Billing(Read), state = client, user = me)]
    pub async fn overrides(
        State(client): State<super::Client>,
        CurrentUser(me): CurrentUser<super::User>,
    ) -> Result<String, Error> {
        let n = client.effective_grants(&me.id)?.global.len();
        Ok(format!("{} has {n}", granted.user_id()))
    }

    #[air_tight::permission(Billing(Manage), kind = fn)]
    pub fn admin_double(x: u32) -> Result<u32, Error> {
        Ok(x * 2)
    }

    #[air_tight::permission(Workspace(Update), kind = fn, resources = [ws])]
    pub async fn rename_ws(ws: super::WsRef) -> Result<String, Error> {
        Ok(format!("renamed {} for {}", ws.0, granted.user_id()))
    }

    #[air_tight::permission(Billing(Manage), kind = fn)]
    pub async fn via_service(
        State(client): State<super::Client>,
        CurrentUser(me): CurrentUser<super::User>,
    ) -> Result<String, Error> {
        let n = admin_double(&client, &me, Vec::new(), 21)?;
        Ok(n.to_string())
    }
}

/// A workspace reference for the fn-mode test.
pub struct WsRef(pub String);

impl IdentifyResources for WsRef {
    fn resource_scopes(&self) -> Vec<ResourceScope> {
        vec![ResourceScope::new(
            ResourceKindKey::from_static("Workspace"),
            &self.0,
        )]
    }
}

fn router() -> Router<Client> {
    Router::new()
        .route("/health", routing::get(handlers::health))
        .route("/ws", routing::post(handlers::read_ws))
        .route("/invoices", routing::post(handlers::create_invoices))
        .route(
            "/billing",
            routing::get(handlers::billing).post(handlers::odd),
        )
        .route("/both", routing::get(handlers::both_billing))
        .route("/who", routing::get(handlers::who))
        .route("/overrides", routing::get(handlers::overrides))
}

async fn auth(
    mut req: axum::extract::Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    if let Some(id) = req.headers().get("x-user").and_then(|v| v.to_str().ok()) {
        let user = User { id: id.to_owned() };
        req.extensions_mut().insert(user);
    }
    next.run(req).await
}

fn app(client: &Client) -> axum::Router {
    router()
        .layer(axum::middleware::from_fn(auth))
        .with_state(client.clone())
        .into_axum()
}

fn client() -> Option<Client> {
    let pool = common::pool()?;
    Some(
        AirTight::builder(pool)
            .notify_channel(&format!("t_{}", uuid::Uuid::new_v4().simple()))
            .resource_kinds::<Kinds>()
            .build()
            .expect("build client"),
    )
}

async fn call(
    app: &axum::Router,
    method: &str,
    uri: &str,
    user: Option<&str>,
    body: Option<&str>,
) -> (StatusCode, String) {
    let mut req = Request::builder().method(method).uri(uri);
    if let Some(u) = user {
        req = req.header("x-user", u);
    }
    let req = match body {
        Some(b) => req
            .header("content-type", "application/json")
            .body(Body::from(b.to_owned())),
        None => req.body(Body::empty()),
    }
    .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    let status = res.status();
    let bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    (status, String::from_utf8_lossy(&bytes).into_owned())
}

fn key(s: &str) -> PermissionKey {
    PermissionKey::new(s)
}

fn ws(id: &str) -> Option<ResourceScope> {
    Some(ResourceScope::new(
        ResourceKindKey::from_static("Workspace"),
        id,
    ))
}

fn inv(id: &str) -> Option<ResourceScope> {
    Some(ResourceScope::new(
        ResourceKindKey::from_static("Invoice"),
        id,
    ))
}

#[test]
fn registry_has_entries() {
    let entry = registry::find(&format!("{}::handlers::read_ws", module_path!()))
        .expect("read_ws registered");
    assert_eq!(entry.kind, registry::HandlerKind::Protected);
    assert_eq!(entry.condition, "Workspace(Read)");
    assert_eq!((entry.keys)(), ["Workspace::Read"]);
    assert_eq!((entry.resource_kinds)(), ["Workspace"]);

    let entry = registry::find(&format!("{}::handlers::odd", module_path!())).unwrap();
    assert_eq!(
        entry.condition,
        "any(Billing(Manage), all(Billing(Read), !Workspace(Update)))"
    );
    assert_eq!(
        (entry.keys)(),
        ["Billing::Manage", "Billing::Read", "Workspace::Update"]
    );

    let health = registry::find(&format!("{}::handlers::health", module_path!())).unwrap();
    assert_eq!(health.kind, registry::HandlerKind::Public);
    assert!(registry::find("nope::nothing").is_none());
}

#[test]
fn protected_marker() {
    use air_tight::router::Protected;
    assert!(handlers::__Protected_read_ws::HANDLER.ends_with("handlers::read_ws"));
    assert_eq!(handlers::__Protected_billing::CONDITION, "Billing(Read)");
}

#[test]
fn router_records_routes_and_audit_passes() {
    let r = router();
    let recorded: Vec<(String, String)> = r
        .routes()
        .iter()
        .map(|r| {
            (
                format!("{} {}", r.method, r.path),
                r.handler.rsplit("::").next().unwrap().to_owned(),
            )
        })
        .collect();
    assert!(recorded.contains(&("POST /billing".into(), "odd".into())));
    assert!(recorded.contains(&("GET /billing".into(), "billing".into())));
    assert_eq!(recorded.len(), 8);
    registry::audit_routes(&r);

    let nested: Router<Client> = Router::new().nest("/api", router());
    assert!(nested.routes().iter().any(|r| r.path == "/api/health"));
}

fn sneaky<S: Clone + Send + Sync + 'static>()
-> air_tight::Guarded<impl axum::handler::Handler<((),), S>> {
    air_tight::Guarded::__new(|| async { "sneaky" })
}

#[test]
#[should_panic(expected = "route `GET /sneaky` is served by")]
fn audit_panics_on_unregistered_route() {
    let r: Router<()> = Router::new().route("/sneaky", routing::get(sneaky));
    registry::audit_routes(&r);
}

#[tokio::test(flavor = "multi_thread")]
async fn end_to_end() {
    let Some(client) = client() else { return };
    let app = app(&client);
    let alice = common::unique("alice");
    let bob = common::unique("bob");
    let carol = common::unique("carol");
    client.grant(None, &alice, key("*"), None).unwrap();
    client
        .grant(None, &bob, key("Workspace::Read"), ws("w1"))
        .unwrap();
    client
        .grant(None, &bob, key("Workspace::Update"), ws("w1"))
        .unwrap();
    client
        .grant(None, &bob, key("Invoice::Create"), inv("i1"))
        .unwrap();
    client
        .grant(None, &bob, key("Billing::Read"), None)
        .unwrap();

    // Unrestricted.
    assert_eq!(
        call(&app, "GET", "/health", None, None).await,
        (StatusCode::OK, "ok".into())
    );
    // 401 without a principal.
    assert_eq!(
        call(&app, "GET", "/billing", None, None).await.0,
        StatusCode::UNAUTHORIZED
    );
    // 403 and 200 in response mode.
    assert_eq!(
        call(&app, "GET", "/billing", Some(&carol), None).await.0,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        call(&app, "GET", "/billing", Some(&bob), None).await,
        (StatusCode::OK, "billing".into())
    );
    // Wildcard.
    assert_eq!(
        call(&app, "GET", "/billing", Some(&alice), None).await.0,
        StatusCode::OK
    );

    // Body resources, scoped.
    let w1 = r#"{"workspace":"w1","invoices":[]}"#;
    let w2 = r#"{"workspace":"w2","invoices":[]}"#;
    assert_eq!(
        call(&app, "POST", "/ws", Some(&bob), Some(w1)).await,
        (StatusCode::OK, "read w1".into())
    );
    assert_eq!(
        call(&app, "POST", "/ws", Some(&bob), Some(w2)).await.0,
        StatusCode::FORBIDDEN
    );

    // Vec ids: every invoice must be allowed.
    let one = r#"{"workspace":"w1","invoices":["i1"]}"#;
    let two = r#"{"workspace":"w1","invoices":["i1","i2"]}"#;
    assert_eq!(
        call(&app, "POST", "/invoices", Some(&bob), Some(one)).await,
        (StatusCode::OK, "1 invoices".into())
    );
    assert_eq!(
        call(&app, "POST", "/invoices", Some(&bob), Some(two))
            .await
            .0,
        StatusCode::FORBIDDEN
    );

    // `!` and any(): bob has Billing::Read but also Workspace::Update only in
    // scope w1, which does not count globally, so `!Workspace(Update)` holds.
    assert_eq!(
        call(&app, "POST", "/billing", Some(&bob), None).await.0,
        StatusCode::OK
    );
    client
        .grant(None, &bob, key("Workspace::Update"), None)
        .unwrap();
    assert_eq!(
        call(&app, "POST", "/billing", Some(&bob), None).await.0,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        call(&app, "POST", "/billing", Some(&alice), None).await.0,
        StatusCode::OK
    );

    // Action expression inside one category.
    assert_eq!(
        call(&app, "GET", "/both", Some(&bob), None).await.0,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        call(&app, "GET", "/both", Some(&alice), None).await.0,
        StatusCode::OK
    );

    // granted.require in the body.
    assert_eq!(
        call(&app, "GET", "/who", Some(&bob), None).await.1,
        "reader"
    );
    assert_eq!(
        call(&app, "GET", "/who", Some(&alice), None).await.1,
        "manager"
    );

    // state/user overrides.
    let (status, body) = call(&app, "GET", "/overrides", Some(&bob), None).await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.starts_with(&format!("{bob} has ")), "{body}");
}

#[tokio::test(flavor = "multi_thread")]
async fn function_mode() {
    let Some(client) = client() else { return };
    let boss = User {
        id: common::unique("boss"),
    };
    let peon = User {
        id: common::unique("peon"),
    };
    client
        .grant(None, &boss.id, key("Billing::Manage"), None)
        .unwrap();
    client
        .grant(None, &peon.id, key("Workspace::Update"), ws("w7"))
        .unwrap();

    assert_eq!(
        handlers::admin_double(&client, &boss, Vec::new(), 4).unwrap(),
        8
    );
    assert!(matches!(
        handlers::admin_double(&client, &peon, Vec::new(), 4),
        Err(Error::Forbidden { .. })
    ));

    let ok = handlers::rename_ws(&client, &peon, Vec::new(), WsRef("w7".into()))
        .await
        .unwrap();
    assert_eq!(ok, format!("renamed w7 for {}", peon.id));
    assert!(
        handlers::rename_ws(&client, &peon, Vec::new(), WsRef("w8".into()))
            .await
            .is_err()
    );
    // Extra scopes passed by the caller are checked too.
    let extra = vec![ResourceScope::new(
        ResourceKindKey::from_static("Workspace"),
        "w9",
    )];
    assert!(
        handlers::rename_ws(&client, &peon, extra, WsRef("w7".into()))
            .await
            .is_err()
    );

    // A fn-mode function calling another one.
    let state = axum::extract::State(client.clone());
    let me = air_tight::CurrentUser(boss.clone());
    assert_eq!(
        handlers::via_service(&client, &boss, Vec::new(), state, me)
            .await
            .unwrap(),
        "42"
    );
}
