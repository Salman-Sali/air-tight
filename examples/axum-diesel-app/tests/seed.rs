//! The demo seed. This is the only test that uses the fixed seed rows.

mod common;

use axum::http::StatusCode;
use axum_diesel_app::AppState;
use axum_diesel_app::auth::UserId;
use axum_diesel_app::seed::{ALICE, BOB, CAROL, INVOICE_A, WORKSPACE_A, WORKSPACE_B, seed};
use common::test_app;

#[tokio::test(flavor = "multi_thread")]
async fn seed_is_idempotent_and_grants_the_documented_access() {
    let Some(t) = test_app() else { return };
    let state = AppState::new(t.db.clone(), t.client.clone());
    seed(&state).unwrap();
    seed(&state).unwrap();

    let roles = t.client.list_roles().unwrap();
    assert_eq!(roles.iter().filter(|r| r.name == "Member").count(), 1);

    let alice = UserId::from_uuid(ALICE);
    let bob = UserId::from_uuid(BOB);
    let carol = UserId::from_uuid(CAROL);
    let a = format!("/workspaces/{WORKSPACE_A}");
    let b = format!("/workspaces/{WORKSPACE_B}");

    for path in [
        &a,
        &b,
        &"/billing".to_owned(),
        &format!("/invoices/{INVOICE_A}"),
    ] {
        assert_eq!(t.get(path, alice).await, StatusCode::OK, "alice {path}");
    }
    assert_eq!(t.get(&a, bob).await, StatusCode::OK);
    assert_eq!(t.get(&b, bob).await, StatusCode::FORBIDDEN);
    assert_eq!(t.get("/billing", bob).await, StatusCode::FORBIDDEN);
    assert_eq!(t.get(&a, carol).await, StatusCode::FORBIDDEN);
    assert_eq!(t.get("/billing", carol).await, StatusCode::FORBIDDEN);

    let grants = t.client.effective_grants(&BOB.to_string()).unwrap();
    assert_eq!(grants.scoped.len(), 2, "{grants:?}");
    assert!(grants.global.is_empty());
}
