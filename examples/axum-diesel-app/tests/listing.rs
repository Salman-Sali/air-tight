//! List filtering: `GET /workspaces` (accessible_ids) and `GET /workspaces/sql`
//! (the diesel `accessible` expression) must agree.

mod common;

use axum::http::StatusCode;
use common::{ids, test_app, ws};

const BOTH_ENDPOINTS: [&str; 2] = ["/workspaces", "/workspaces/sql"];

#[tokio::test(flavor = "multi_thread")]
async fn none_some_and_all() {
    let Some(t) = test_app() else { return };
    let (w1, w2, w3) = (t.workspace(), t.workspace(), t.workspace());

    let nobody = t.user("nobody");
    let some = t.user("some");
    t.grant(some, "Workspace::Read", ws(w1));
    t.grant(some, "Workspace::Read", ws(w2));
    // A grant for another action does not count.
    t.grant(some, "Workspace::Update", ws(w3));
    let all = t.user("all");
    t.grant(all, "Workspace::Read", None);

    for path in BOTH_ENDPOINTS {
        let (status, body) = t.send("GET", path, Some(nobody), None).await;
        assert_eq!(status, StatusCode::OK, "{path}");
        assert!(ids(&body).is_empty(), "{path}: {body}");

        let (status, body) = t.send("GET", path, Some(some), None).await;
        assert_eq!(status, StatusCode::OK, "{path}");
        let mut expected = vec![w1.to_string(), w2.to_string()];
        expected.sort();
        assert_eq!(ids(&body), expected, "{path}");

        let (status, body) = t.send("GET", path, Some(all), None).await;
        assert_eq!(status, StatusCode::OK, "{path}");
        let seen = ids(&body);
        for w in [w1, w2, w3] {
            assert!(seen.contains(&w.to_string()), "{path}: missing {w}");
        }
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn wildcards_roles_and_odd_scope_ids() {
    let Some(t) = test_app() else { return };
    let (w1, w2) = (t.workspace(), t.workspace());

    // Global wildcard sees everything, on both paths.
    let star = t.user("star");
    t.grant(star, "*", None);
    let category = t.user("category");
    t.grant(category, "Workspace::*", None);
    for user in [star, category] {
        for path in BOTH_ENDPOINTS {
            let (_, body) = t.send("GET", path, Some(user), None).await;
            let seen = ids(&body);
            assert!(
                seen.contains(&w1.to_string()) && seen.contains(&w2.to_string()),
                "{path}"
            );
        }
    }

    // A scoped role assignment plus a scope id that is not a uuid.
    let member = t.user("member");
    let role = t.role(&["Workspace::Read"]);
    t.client
        .assign_role(None, &member.to_string(), role, ws(w2))
        .unwrap();
    t.client
        .grant(
            None,
            &member.to_string(),
            air_tight::PermissionKey::new("Workspace::Read"),
            Some(air_tight::ResourceScope::new(
                air_tight::ResourceKindKey::new("Workspace"),
                "not-a-uuid",
            )),
        )
        .unwrap();
    for path in BOTH_ENDPOINTS {
        let (status, body) = t.send("GET", path, Some(member), None).await;
        assert_eq!(status, StatusCode::OK, "{path}");
        assert_eq!(ids(&body), vec![w2.to_string()], "{path}");
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn the_cached_list_follows_role_changes() {
    let Some(t) = test_app() else { return };
    let w = t.workspace();
    let user = t.user("member");
    let role = t.role(&["Workspace::Read"]);
    t.client
        .assign_role(None, &user.to_string(), role, ws(w))
        .unwrap();

    let (_, body) = t.send("GET", "/workspaces", Some(user), None).await;
    assert_eq!(ids(&body), vec![w.to_string()]);

    t.set_role(role, &["Invoice::Read"]);
    for path in BOTH_ENDPOINTS {
        let (_, body) = t.send("GET", path, Some(user), None).await;
        assert!(ids(&body).is_empty(), "{path}");
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn lists_require_authentication() {
    let Some(t) = test_app() else { return };
    for path in BOTH_ENDPOINTS {
        assert_eq!(
            t.send("GET", path, None, None).await.0,
            StatusCode::UNAUTHORIZED
        );
    }
}
