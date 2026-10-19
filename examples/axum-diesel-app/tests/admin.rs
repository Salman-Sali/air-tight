//! The admin endpoints: thin wrappers over the management API.

mod common;

use air_tight::schema::audit_log;
use axum::http::StatusCode;
use common::test_app;
use diesel::prelude::*;
use serde_json::json;
use uuid::Uuid;

#[tokio::test(flavor = "multi_thread")]
async fn admin_endpoints_need_billing_manage() {
    let Some(t) = test_app() else { return };
    let nobody = t.user("nobody");
    let reader = t.user("billing-reader");
    t.grant(reader, "Billing::Read", None);
    let role = json!({ "name": format!("r-{}", Uuid::new_v4()) });
    let assign = json!({ "user_id": "x", "role_id": Uuid::new_v4() });
    let grant = json!({ "user_id": "x", "permission": "Billing::Read" });

    for (path, body) in [
        ("/admin/roles", &role),
        ("/admin/assign", &assign),
        ("/admin/grant", &grant),
    ] {
        assert_eq!(
            t.send("POST", path, None, Some(body.clone())).await.0,
            StatusCode::UNAUTHORIZED
        );
        for user in [nobody, reader] {
            assert_eq!(
                t.send("POST", path, Some(user), Some(body.clone())).await.0,
                StatusCode::FORBIDDEN,
                "{path}"
            );
        }
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn create_assign_and_grant_end_to_end() {
    let Some(t) = test_app() else { return };
    let admin = t.user("admin");
    t.grant(admin, "Billing::Manage", None);
    let target = t.user("target");
    let w = t.workspace();
    let path = format!("/workspaces/{w}");
    assert_eq!(t.get(&path, target).await, StatusCode::FORBIDDEN);

    // Create a role.
    let name = format!("viewers-{}", Uuid::new_v4());
    let (status, body) = t
        .send(
            "POST",
            "/admin/roles",
            Some(admin),
            Some(json!({ "name": name, "description": "read workspaces", "permissions": ["Workspace::Read"] })),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "{body}");
    let role_id = body["id"].as_str().unwrap().to_owned();
    let roles = t.client.list_roles().unwrap();
    let created = roles.iter().find(|r| r.name == name).expect("role exists");
    assert_eq!(created.id.to_string(), role_id);
    assert_eq!(created.permissions.len(), 1);

    // Assign it within the workspace; the target can now read it.
    let (status, _) = t
        .send(
            "POST",
            "/admin/assign",
            Some(admin),
            Some(json!({ "user_id": target.to_string(), "role_id": role_id, "scope": { "kind": "Workspace", "id": w } })),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(t.get(&path, target).await, StatusCode::OK);
    assert_eq!(
        t.get(&format!("/workspaces/{}", t.workspace()), target)
            .await,
        StatusCode::FORBIDDEN
    );

    // Grant directly, globally.
    let (status, _) = t
        .send(
            "POST",
            "/admin/grant",
            Some(admin),
            Some(json!({ "user_id": target.to_string(), "permission": "Billing::Read" })),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(t.get("/billing", target).await, StatusCode::OK);

    // The actor is recorded in the audit log.
    let mut conn = t.db.pool().get().unwrap();
    let actions: Vec<String> = audit_log::table
        .filter(audit_log::actor_user_id.eq(admin.to_string()))
        .select(audit_log::action)
        .load(&mut conn)
        .unwrap();
    for expected in [
        "role_created",
        "role_permissions_set",
        "role_assigned",
        "grant_added",
    ] {
        assert!(
            actions.iter().any(|a| a == expected),
            "missing {expected} in {actions:?}"
        );
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn bad_input_is_rejected_cleanly() {
    let Some(t) = test_app() else { return };
    let admin = t.user("admin");
    t.grant(admin, "*", None);
    let name = format!("dup-{}", Uuid::new_v4());

    let create = |permissions: serde_json::Value, name: String| json!({ "name": name, "permissions": permissions });
    let (status, _) = t
        .send(
            "POST",
            "/admin/roles",
            Some(admin),
            Some(create(json!([]), name.clone())),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED);
    let (status, _) = t
        .send(
            "POST",
            "/admin/roles",
            Some(admin),
            Some(create(json!([]), name)),
        )
        .await;
    assert_eq!(status, StatusCode::CONFLICT);
    let (status, _) = t
        .send(
            "POST",
            "/admin/roles",
            Some(admin),
            Some(create(
                json!(["Nope::Nope"]),
                format!("n-{}", Uuid::new_v4()),
            )),
        )
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let (status, _) = t
        .send(
            "POST",
            "/admin/roles",
            Some(admin),
            Some(create(json!([]), "   ".to_owned())),
        )
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    let (status, _) = t
        .send(
            "POST",
            "/admin/grant",
            Some(admin),
            Some(json!({ "user_id": "x", "permission": "Billing::Read", "scope": { "kind": "Planet", "id": "mars" } })),
        )
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let (status, _) = t
        .send(
            "POST",
            "/admin/grant",
            Some(admin),
            Some(json!({ "user_id": "x", "permission": "Nope::Nope" })),
        )
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test(flavor = "multi_thread")]
async fn granting_through_the_api_invalidates_the_cache() {
    let Some(t) = test_app() else { return };
    let admin = t.user("admin");
    t.grant(admin, "Billing::Manage", None);
    let target = t.user("target");
    let w = t.workspace();
    let path = format!("/workspaces/{w}");

    // Warm the target's cache with an empty grant set.
    assert_eq!(t.get(&path, target).await, StatusCode::FORBIDDEN);

    let (status, _) = t
        .send(
            "POST",
            "/admin/grant",
            Some(admin),
            Some(json!({ "user_id": target.to_string(), "permission": "Workspace::Read", "scope": { "kind": "Workspace", "id": w } })),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(t.get(&path, target).await, StatusCode::OK);
}
