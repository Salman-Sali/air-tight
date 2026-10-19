//! Body-identified resources: single ids, `&&`, `all(..)` and Vec ids.

mod common;

use axum::http::StatusCode;
use common::{inv, test_app, ws};
use serde_json::json;
use uuid::Uuid;

#[tokio::test(flavor = "multi_thread")]
async fn create_invoice_needs_both_permissions() {
    let Some(t) = test_app() else { return };
    let w = t.workspace();
    let body = json!({ "workspace_id": w, "amount": 250 });

    let only_workspace = t.user("workspace-only");
    t.grant(only_workspace, "Workspace::Update", ws(w));
    let only_invoice = t.user("invoice-only");
    t.grant(only_invoice, "Invoice::Create", None);
    let both = t.user("both");
    t.grant(both, "Workspace::Update", ws(w));
    t.grant(both, "Invoice::Create", None);
    let wrong_workspace = t.user("wrong-workspace");
    t.grant(wrong_workspace, "Workspace::Update", ws(t.workspace()));
    t.grant(wrong_workspace, "Invoice::Create", None);

    for user in [only_workspace, only_invoice, wrong_workspace] {
        let (status, _) = t
            .send("POST", "/invoices", Some(user), Some(body.clone()))
            .await;
        assert_eq!(status, StatusCode::FORBIDDEN);
    }
    let (status, created) = t.send("POST", "/invoices", Some(both), Some(body)).await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(created["workspace_id"], json!(w.to_string()));
    assert_eq!(created["amount"], json!(250));
}

#[tokio::test(flavor = "multi_thread")]
async fn create_invoice_in_unknown_workspace_is_404_for_authorized_callers() {
    let Some(t) = test_app() else { return };
    let admin = t.user("admin");
    t.grant(admin, "*", None);
    let body = json!({ "workspace_id": Uuid::new_v4(), "amount": 1 });
    let (status, _) = t.send("POST", "/invoices", Some(admin), Some(body)).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test(flavor = "multi_thread")]
async fn move_invoice_checks_every_id_including_the_vec() {
    let Some(t) = test_app() else { return };
    let source = t.workspace();
    let target = t.workspace();
    let (i1, i2, i3) = (t.invoice(source), t.invoice(source), t.invoice(source));

    let user = t.user("mover");
    t.grant(user, "Invoice::Update", inv(i1));
    t.grant(user, "Invoice::Update", inv(i2));
    t.grant(user, "Workspace::Update", ws(target));

    // i3 is not covered: the whole request is refused and nothing moves.
    let refused = json!({
        "invoice_id": i1,
        "related_invoice_ids": [i2, i3],
        "target_workspace_id": target,
    });
    let (status, _) = t
        .send("POST", "/invoices/move", Some(user), Some(refused))
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(t.invoice_workspace(i1), source);
    assert_eq!(t.invoice_workspace(i2), source);

    // Every id covered: allowed, and all of them move.
    let allowed = json!({
        "invoice_id": i1,
        "related_invoice_ids": [i2],
        "target_workspace_id": target,
    });
    let (status, body) = t
        .send("POST", "/invoices/move", Some(user), Some(allowed))
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["moved"], json!(2));
    assert_eq!(t.invoice_workspace(i1), target);
    assert_eq!(t.invoice_workspace(i2), target);
    assert_eq!(t.invoice_workspace(i3), source);

    // The related list may be omitted.
    let (status, _) = t
        .send(
            "POST",
            "/invoices/move",
            Some(user),
            Some(json!({ "invoice_id": i1, "target_workspace_id": target })),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test(flavor = "multi_thread")]
async fn move_invoice_needs_the_target_workspace_too() {
    let Some(t) = test_app() else { return };
    let source = t.workspace();
    let target = t.workspace();
    let i1 = t.invoice(source);
    let user = t.user("half");
    t.grant(user, "Invoice::Update", inv(i1));
    t.grant(user, "Workspace::Update", ws(source));

    let body = json!({ "invoice_id": i1, "target_workspace_id": target });
    let (status, _) = t
        .send("POST", "/invoices/move", Some(user), Some(body))
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    // A global wildcard covers everything.
    let admin = t.user("admin");
    t.grant(admin, "*", None);
    let body =
        json!({ "invoice_id": i1, "related_invoice_ids": [], "target_workspace_id": target });
    let (status, _) = t
        .send("POST", "/invoices/move", Some(admin), Some(body))
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(t.invoice_workspace(i1), target);
}

#[tokio::test(flavor = "multi_thread")]
async fn malformed_body_is_a_client_error_not_a_permission_error() {
    let Some(t) = test_app() else { return };
    let admin = t.user("admin");
    t.grant(admin, "*", None);
    let (status, _) = t
        .send("POST", "/invoices", Some(admin), Some(json!({ "nope": 1 })))
        .await;
    assert!(
        status.is_client_error() && status != StatusCode::FORBIDDEN,
        "{status}"
    );
}
