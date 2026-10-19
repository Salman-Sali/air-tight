//! Authentication, authorization outcomes, scoping, wildcards, invalidation
//! and condition operators, end to end through the router.

mod common;

use axum::http::StatusCode;
use common::{inv, test_app, ws};
use serde_json::json;
use uuid::Uuid;

#[tokio::test(flavor = "multi_thread")]
async fn health_is_unrestricted() {
    let Some(t) = test_app() else { return };
    let (status, body) = t.send("GET", "/health", None, None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, json!("ok"));
}

#[tokio::test(flavor = "multi_thread")]
async fn unauthenticated_requests_get_401() {
    let Some(t) = test_app() else { return };
    let w = t.workspace();
    let path = format!("/workspaces/{w}");

    // No header at all.
    assert_eq!(
        t.send("GET", &path, None, None).await.0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        t.send("GET", "/workspaces", None, None).await.0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        t.send("GET", "/billing", None, None).await.0,
        StatusCode::UNAUTHORIZED
    );

    // Malformed id and unknown id.
    for header in ["not-a-uuid", &Uuid::new_v4().to_string()] {
        let req = axum::http::Request::builder()
            .uri(&path)
            .header("x-user-id", header)
            .body(axum::body::Body::empty())
            .unwrap();
        let res = tower::ServiceExt::oneshot(t.app.clone(), req)
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED, "{header}");
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn forbidden_then_ok_when_granted() {
    let Some(t) = test_app() else { return };
    let user = t.user("carol");
    let w = t.workspace();
    let path = format!("/workspaces/{w}");

    assert_eq!(t.get(&path, user).await, StatusCode::FORBIDDEN);

    t.grant(user, "Workspace::Read", ws(w));
    let (status, body) = t.send("GET", &path, Some(user), None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["id"], json!(w.to_string()));
}

#[tokio::test(flavor = "multi_thread")]
async fn scoped_grant_covers_only_its_resource_but_global_covers_all() {
    let Some(t) = test_app() else { return };
    let (w1, w2) = (t.workspace(), t.workspace());
    let scoped = t.user("scoped");
    let global = t.user("global");
    t.grant(scoped, "Workspace::Read", ws(w1));
    t.grant(global, "Workspace::Read", None);

    assert_eq!(
        t.get(&format!("/workspaces/{w1}"), scoped).await,
        StatusCode::OK
    );
    assert_eq!(
        t.get(&format!("/workspaces/{w2}"), scoped).await,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        t.get(&format!("/workspaces/{w1}"), global).await,
        StatusCode::OK
    );
    assert_eq!(
        t.get(&format!("/workspaces/{w2}"), global).await,
        StatusCode::OK
    );

    // A different action is not covered by a Read grant.
    let body = Some(json!({ "name": "renamed" }));
    let (status, _) = t
        .send("PATCH", &format!("/workspaces/{w1}"), Some(scoped), body)
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[tokio::test(flavor = "multi_thread")]
async fn wildcards_match() {
    let Some(t) = test_app() else { return };
    let w = t.workspace();
    let everything = t.user("star");
    let category = t.user("category");
    let other_category = t.user("other");
    t.grant(everything, "*", None);
    t.grant(category, "Workspace::*", None);
    t.grant(other_category, "Invoice::*", None);

    let path = format!("/workspaces/{w}");
    assert_eq!(t.get(&path, everything).await, StatusCode::OK);
    assert_eq!(t.get(&path, category).await, StatusCode::OK);
    assert_eq!(t.get(&path, other_category).await, StatusCode::FORBIDDEN);
    assert_eq!(t.get("/billing", everything).await, StatusCode::OK);
    assert_eq!(t.get("/billing", category).await, StatusCode::FORBIDDEN);

    // A scoped category wildcard.
    let scoped = t.user("scoped-wildcard");
    t.grant(scoped, "Workspace::*", ws(w));
    assert_eq!(t.get(&path, scoped).await, StatusCode::OK);
    assert_eq!(
        t.get(&format!("/workspaces/{}", t.workspace()), scoped)
            .await,
        StatusCode::FORBIDDEN
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn role_changes_are_visible_on_the_next_request() {
    let Some(t) = test_app() else { return };
    let w = t.workspace();
    let user = t.user("member");
    let role = t.role(&["Workspace::Read"]);
    t.client
        .assign_role(None, &user.to_string(), role, ws(w))
        .unwrap();
    let path = format!("/workspaces/{w}");

    // Warm the cache.
    assert_eq!(t.get(&path, user).await, StatusCode::OK);
    assert_eq!(t.get(&path, user).await, StatusCode::OK);

    // Replace the role's permissions: the cached grants must not survive.
    t.set_role(role, &["Invoice::Read"]);
    assert_eq!(t.get(&path, user).await, StatusCode::FORBIDDEN);

    t.set_role(role, &["Workspace::Read", "Workspace::Update"]);
    assert_eq!(t.get(&path, user).await, StatusCode::OK);

    // Revoking the assignment is visible too.
    t.client
        .revoke_role(None, &user.to_string(), role, ws(w))
        .unwrap();
    assert_eq!(t.get(&path, user).await, StatusCode::FORBIDDEN);
}

#[tokio::test(flavor = "multi_thread")]
async fn not_and_and_conditions() {
    let Some(t) = test_app() else { return };
    // Billing(Read) && !Billing(Manage)
    let nobody = t.user("nobody");
    let reader = t.user("reader");
    let manager = t.user("manager");
    t.grant(reader, "Billing::Read", None);
    t.grant(manager, "Billing::Read", None);
    t.grant(manager, "Billing::Manage", None);

    assert_eq!(
        t.get("/billing/readonly", nobody).await,
        StatusCode::FORBIDDEN
    );
    assert_eq!(t.get("/billing/readonly", reader).await, StatusCode::OK);
    assert_eq!(
        t.get("/billing/readonly", manager).await,
        StatusCode::FORBIDDEN
    );

    // Granting Manage later flips the answer for the same user.
    t.grant(reader, "Billing::Manage", None);
    assert_eq!(
        t.get("/billing/readonly", reader).await,
        StatusCode::FORBIDDEN
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn or_condition() {
    let Some(t) = test_app() else { return };
    // Invoice(Read) || Workspace(Update)
    let (w, i1) = {
        let w = t.workspace();
        (w, t.invoice(w))
    };
    let i2 = t.invoice(w);
    let reader = t.user("invoice-reader");
    let editor = t.user("workspace-editor");
    let neither = t.user("neither");
    t.grant(reader, "Invoice::Read", inv(i1));
    t.grant(editor, "Workspace::Update", None);
    // Workspace::Update scoped to one workspace does not satisfy the global leaf.
    t.grant(neither, "Workspace::Update", ws(w));

    let p1 = format!("/invoices/{i1}");
    let p2 = format!("/invoices/{i2}");
    assert_eq!(t.get(&p1, reader).await, StatusCode::OK);
    assert_eq!(t.get(&p2, reader).await, StatusCode::FORBIDDEN);
    assert_eq!(t.get(&p1, editor).await, StatusCode::OK);
    assert_eq!(t.get(&p2, editor).await, StatusCode::OK);
    assert_eq!(t.get(&p1, neither).await, StatusCode::FORBIDDEN);
}

#[tokio::test(flavor = "multi_thread")]
async fn granted_require_inside_the_handler() {
    let Some(t) = test_app() else { return };
    let reader = t.user("reader");
    let manager = t.user("manager");
    t.grant(reader, "Billing::Read", None);
    t.grant(manager, "Billing::Manage", None);
    t.grant(manager, "Billing::Read", None);

    let (status, body) = t.send("GET", "/billing", Some(reader), None).await;
    assert_eq!(
        (status, &body),
        (StatusCode::OK, &json!({ "can_manage": false }))
    );
    let (status, body) = t.send("GET", "/billing", Some(manager), None).await;
    assert_eq!(
        (status, &body),
        (StatusCode::OK, &json!({ "can_manage": true }))
    );
    assert_eq!(
        t.get("/billing", t.user("nobody")).await,
        StatusCode::FORBIDDEN
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn authorization_happens_before_existence() {
    let Some(t) = test_app() else { return };
    let nobody = t.user("nobody");
    let admin = t.user("admin");
    t.grant(admin, "*", None);
    let missing = format!("/workspaces/{}", Uuid::new_v4());

    // No permission: 403 even though the workspace does not exist.
    assert_eq!(t.get(&missing, nobody).await, StatusCode::FORBIDDEN);
    // With permission the handler runs and reports the missing row.
    assert_eq!(t.get(&missing, admin).await, StatusCode::NOT_FOUND);
}
