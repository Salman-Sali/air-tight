//! Tests of `AirTightPath` and `AirTightQuery`: extraction, the resource
//! scopes they report, and their use with `#[air_tight::permission]`. Tests
//! needing a database skip when `AIR_TIGHT_TEST_DATABASE_URL` is unset.

mod common;

use air_tight::{
    AirTight, AirTightPath, AirTightQuery, IdentifyResources, Permission, PermissionCategory,
    PermissionKey, Principal, ResourceKind, ResourceKindKey, ResourceScope, Router, routing,
};
use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde::Deserialize;
use tower::ServiceExt;

use res_kinds::{Invoice, Workspace};

#[derive(ResourceKind)]
pub enum Res {
    Workspace,
    Invoice,
    Item,
}

#[derive(Permission)]
pub enum Act {
    Read,
    Update,
}

#[derive(PermissionCategory)]
pub enum Perms {
    #[air_tight(resource = res_kinds::Item)]
    Item(Act),
    #[air_tight(resource = res_kinds::Workspace)]
    Workspace(Act),
    #[air_tight(resource = res_kinds::Invoice)]
    Invoice(Act),
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

air_tight::setup!(perms = Perms, user = User, resources = Res);

type Client = AirTight<Perms, User>;

#[derive(Debug, Deserialize, IdentifyResources)]
pub struct Filter {
    #[air_tight(resource = Workspace)]
    pub ws: String,
    pub q: Option<String>,
}

fn show(scopes: Vec<ResourceScope>) -> String {
    scopes
        .iter()
        .map(|s| format!("{}={}", s.kind, s.id))
        .collect::<Vec<_>>()
        .join(",")
}

mod plain {
    use super::*;

    pub async fn one(p: AirTightPath<String, Workspace>) -> String {
        show(p.resource_scopes())
    }

    pub async fn two(p: AirTightPath<(String, u32), (Workspace, Invoice)>) -> String {
        format!("{:?} {}", p.clone().into_inner(), show(p.resource_scopes()))
    }

    pub async fn query(q: AirTightQuery<Filter, ()>) -> String {
        format!("{} {:?}", show(q.resource_scopes()), q.q)
    }
}

mod guarded {
    use super::*;

    #[air_tight::permission(Item(Read))]
    pub async fn item(path: AirTightPath<String, Item>) -> String {
        format!("item {}", *path)
    }

    #[air_tight::permission(Workspace(Update) && Invoice(Read))]
    pub async fn pair(p: AirTightPath<(String, String), (Workspace, Invoice)>) -> String {
        let (ws, inv) = p.into_inner();
        format!("{ws}/{inv}")
    }

    #[air_tight::permission(Workspace(Read))]
    pub async fn search(q: AirTightQuery<Filter, ()>) -> String {
        format!("search {}", q.ws)
    }
}

use res_kinds::Item;

async fn get(app: &axum::Router, uri: &str, user: Option<&str>) -> (StatusCode, String) {
    let mut req = Request::builder().uri(uri);
    if let Some(u) = user {
        req = req.header("x-user", u);
    }
    let res = app
        .clone()
        .oneshot(req.body(Body::empty()).unwrap())
        .await
        .unwrap();
    let status = res.status();
    let bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    (status, String::from_utf8_lossy(&bytes).into_owned())
}

fn plain_app() -> axum::Router {
    axum::Router::new()
        .route("/one/{id}", axum::routing::get(plain::one))
        .route("/two/{ws}/{n}", axum::routing::get(plain::two))
        .route("/q", axum::routing::get(plain::query))
}

#[tokio::test]
async fn path_and_query_extract_and_identify() {
    let app = plain_app();
    assert_eq!(
        get(&app, "/one/w1", None).await,
        (StatusCode::OK, "Workspace=w1".into())
    );
    assert_eq!(
        get(&app, "/two/w2/7", None).await,
        (StatusCode::OK, "(\"w2\", 7) Workspace=w2,Invoice=7".into())
    );
    assert_eq!(
        get(&app, "/q?ws=w3&q=hello", None).await,
        (StatusCode::OK, "Workspace=w3 Some(\"hello\")".into())
    );
}

#[tokio::test]
async fn rejections_are_client_errors() {
    let app = plain_app();
    assert_eq!(
        get(&app, "/two/w2/not-a-number", None).await.0,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(get(&app, "/q", None).await.0, StatusCode::BAD_REQUEST);
}

fn guarded_app(client: &Client) -> axum::Router {
    Router::<Client>::new()
        .route("/items/{id}", routing::get(guarded::item))
        .route("/{ws}/{inv}", routing::get(guarded::pair))
        .route("/search", routing::get(guarded::search))
        .layer(axum::middleware::from_fn(
            |mut req: axum::extract::Request, next: axum::middleware::Next| async move {
                if let Some(id) = req.headers().get("x-user").and_then(|v| v.to_str().ok()) {
                    let user = User { id: id.to_owned() };
                    req.extensions_mut().insert(user);
                }
                next.run(req).await
            },
        ))
        .with_state(client.clone())
        .into_axum()
}

fn key(s: &str) -> PermissionKey {
    PermissionKey::new(s)
}

fn scope(kind: &'static str, id: &str) -> Option<ResourceScope> {
    Some(ResourceScope::new(ResourceKindKey::from_static(kind), id))
}

#[tokio::test(flavor = "multi_thread")]
async fn permission_with_extractors_end_to_end() {
    let Some(pool) = common::pool() else { return };
    common::connect();
    let client: Client = AirTight::builder(pool)
        .notify_channel(&format!("t_{}", uuid::Uuid::new_v4().simple()))
        .resource_kinds::<Res>()
        .build()
        .expect("build client");
    let app = guarded_app(&client);
    let bob = common::unique("bob");
    let carol = common::unique("carol");
    client
        .grant(None, &bob, key("Item::Read"), scope("Item", "i1"))
        .unwrap();
    client
        .grant(
            None,
            &bob,
            key("Workspace::Update"),
            scope("Workspace", "w1"),
        )
        .unwrap();
    client
        .grant(None, &bob, key("Invoice::Read"), scope("Invoice", "v1"))
        .unwrap();
    client
        .grant(None, &bob, key("Workspace::Read"), scope("Workspace", "w1"))
        .unwrap();

    // Auto-included single path param.
    assert_eq!(
        get(&app, "/items/i1", Some(&bob)).await,
        (StatusCode::OK, "item i1".into())
    );
    assert_eq!(
        get(&app, "/items/i2", Some(&bob)).await.0,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        get(&app, "/items/i1", Some(&carol)).await.0,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        get(&app, "/items/i1", None).await.0,
        StatusCode::UNAUTHORIZED
    );

    // Two ids with two kinds: both must be granted in scope.
    assert_eq!(
        get(&app, "/w1/v1", Some(&bob)).await,
        (StatusCode::OK, "w1/v1".into())
    );
    assert_eq!(
        get(&app, "/w1/v2", Some(&bob)).await.0,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        get(&app, "/w2/v1", Some(&bob)).await.0,
        StatusCode::FORBIDDEN
    );

    // Query struct with Kinds = ().
    assert_eq!(
        get(&app, "/search?ws=w1", Some(&bob)).await,
        (StatusCode::OK, "search w1".into())
    );
    assert_eq!(
        get(&app, "/search?ws=w2", Some(&bob)).await.0,
        StatusCode::FORBIDDEN
    );
}
