//! `kind = fn` services and the `&Granted` helper, called with and without a
//! router in front.

mod common;

use air_tight::Error;
use axum::http::StatusCode;
use axum_diesel_app::auth::AuthenticatedUser;
use axum_diesel_app::error::ApiError;
use axum_diesel_app::requests::WorkspaceRef;
use axum_diesel_app::services::rename_workspace;
use common::{test_app, ws};
use serde_json::json;
use uuid::Uuid;

fn principal(id: axum_diesel_app::auth::UserId) -> AuthenticatedUser {
    AuthenticatedUser {
        id,
        name: "test".to_owned(),
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn rename_service_checks_its_own_permission() {
    let Some(t) = test_app() else { return };
    let (w1, w2) = (t.workspace(), t.workspace());
    let user = t.user("editor");
    t.grant(user, "Workspace::Update", ws(w1));
    let me = principal(user);

    let renamed = rename_workspace(
        &t.client,
        &me,
        Vec::new(),
        &t.db,
        WorkspaceRef { id: w1 },
        "fresh".into(),
    )
    .await
    .unwrap();
    assert_eq!(renamed.name, "fresh");
    assert_eq!(t.workspace_name(w1), "fresh");

    // Another workspace: forbidden, and the name is untouched.
    let before = t.workspace_name(w2);
    let err = rename_workspace(
        &t.client,
        &me,
        Vec::new(),
        &t.db,
        WorkspaceRef { id: w2 },
        "x".into(),
    )
    .await
    .unwrap_err();
    assert!(
        matches!(err, ApiError::Auth(Error::Forbidden { .. })),
        "{err:?}"
    );
    assert_eq!(t.workspace_name(w2), before);

    // Scopes passed by the caller are checked in addition to the argument's.
    let extra = vec![air_tight::ResourceScope::new(
        air_tight::ResourceKindKey::new("Workspace"),
        w2.to_string(),
    )];
    let err = rename_workspace(
        &t.client,
        &me,
        extra,
        &t.db,
        WorkspaceRef { id: w1 },
        "y".into(),
    )
    .await
    .unwrap_err();
    assert!(matches!(err, ApiError::Auth(Error::Forbidden { .. })));
}

#[tokio::test(flavor = "multi_thread")]
async fn patch_route_uses_the_service() {
    let Some(t) = test_app() else { return };
    let (w1, w2) = (t.workspace(), t.workspace());
    let user = t.user("editor");
    t.grant(user, "Workspace::Update", ws(w1));

    let (status, body) = t
        .send(
            "PATCH",
            &format!("/workspaces/{w1}"),
            Some(user),
            Some(json!({ "name": "via-route" })),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["name"], json!("via-route"));
    assert_eq!(t.workspace_name(w1), "via-route");

    let (status, _) = t
        .send(
            "PATCH",
            &format!("/workspaces/{w2}"),
            Some(user),
            Some(json!({ "name": "nope" })),
        )
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_ne!(t.workspace_name(w2), "nope");

    let (status, _) = t
        .send(
            "PATCH",
            &format!("/workspaces/{}", Uuid::new_v4()),
            Some(user),
            Some(json!({ "name": "n" })),
        )
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[test]
fn billing_summary_uses_the_granted_snapshot() {
    // `Granted` can only come from `authorize`, so this needs the database.
    let Some(t) = test_app() else { return };
    let user = t.user("manager");
    t.grant(user, "Billing::Manage", None);
    let me = principal(user);
    let granted = t
        .client
        .authorize(
            &me,
            &air_tight::Condition::Leaf(axum_diesel_app::authz::MyPerms::Billing(
                axum_diesel_app::authz::BillingAction::Manage,
            )),
            Vec::new(),
        )
        .unwrap();
    assert!(axum_diesel_app::services::billing_summary(&granted).can_manage);
    assert!(axum_diesel_app::services::require_manage(&granted).is_ok());

    // The snapshot is fixed: later revocation does not change this proof.
    t.client
        .revoke(
            None,
            &user.to_string(),
            air_tight::PermissionKey::new("Billing::Manage"),
            None,
        )
        .unwrap();
    assert!(axum_diesel_app::services::require_manage(&granted).is_ok());
}
