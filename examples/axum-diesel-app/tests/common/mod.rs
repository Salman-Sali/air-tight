//! Shared helpers for the integration tests.
//!
//! Tests run in parallel against one shared database, so every test creates
//! its own users, workspaces, invoices and roles with fresh ids and a fresh
//! LISTEN/NOTIFY channel. Only `seed.rs` touches the fixed seed rows.
#![allow(dead_code)]

use std::sync::Once;

use air_tight::{AirTight, PermissionKey, ResourceKind, ResourceKindKey, ResourceScope, RoleName};
use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum_diesel_app::auth::UserId;
use axum_diesel_app::authz::{MyPerms, MyResources};
use axum_diesel_app::db::{Db, connect, migrate};
use axum_diesel_app::models::{NewInvoice, UserRecord, WorkspaceRecord};
use axum_diesel_app::schema::{invoices, users, workspaces};
use axum_diesel_app::{AppState, Client, app};
use diesel::prelude::*;
use serde_json::Value;
use tower::ServiceExt;
use uuid::Uuid;

static MIGRATED: Once = Once::new();

/// The running application plus handles to prepare and inspect data.
pub struct TestApp {
    pub app: axum::Router,
    pub client: Client,
    pub db: Db,
}

/// Builds the app on the test database, or `None` (skip) when
/// `AIR_TIGHT_TEST_DATABASE_URL` is unset.
pub fn test_app() -> Option<TestApp> {
    let Ok(url) = std::env::var("AIR_TIGHT_TEST_DATABASE_URL") else {
        eprintln!("skipping: AIR_TIGHT_TEST_DATABASE_URL is not set");
        return None;
    };
    let pool = connect(&url, 6).expect("connect to the test database");
    MIGRATED.call_once(|| migrate(&mut pool.get().unwrap()).expect("migrate example tables"));
    let client = AirTight::<MyPerms, _>::builder(pool.clone())
        .notify_channel(&format!("t_{}", Uuid::new_v4().simple()))
        .resource_kinds::<MyResources>()
        .build()
        .expect("build the air-tight client");
    let db = Db::new(pool);
    let state = AppState::new(db.clone(), client.clone());
    Some(TestApp {
        app: app(state),
        client,
        db,
    })
}

impl TestApp {
    /// Inserts a user with a fresh id.
    pub fn user(&self, label: &str) -> UserId {
        let id = Uuid::new_v4();
        let row = UserRecord {
            id,
            name: format!("{label}-{id}"),
        };
        let mut conn = self.db.pool().get().unwrap();
        diesel::insert_into(users::table)
            .values(&row)
            .execute(&mut conn)
            .unwrap();
        UserId::from_uuid(id)
    }

    /// Inserts a workspace with a fresh id.
    pub fn workspace(&self) -> Uuid {
        let id = Uuid::new_v4();
        let row = WorkspaceRecord {
            id,
            name: format!("ws-{id}"),
        };
        let mut conn = self.db.pool().get().unwrap();
        diesel::insert_into(workspaces::table)
            .values(&row)
            .execute(&mut conn)
            .unwrap();
        id
    }

    /// Inserts an invoice into `workspace_id`.
    pub fn invoice(&self, workspace_id: Uuid) -> Uuid {
        let id = Uuid::new_v4();
        let mut conn = self.db.pool().get().unwrap();
        diesel::insert_into(invoices::table)
            .values(NewInvoice {
                id,
                workspace_id,
                amount: 100,
            })
            .execute(&mut conn)
            .unwrap();
        id
    }

    /// The workspace an invoice currently belongs to.
    pub fn invoice_workspace(&self, invoice_id: Uuid) -> Uuid {
        let mut conn = self.db.pool().get().unwrap();
        invoices::table
            .find(invoice_id)
            .select(invoices::workspace_id)
            .first(&mut conn)
            .unwrap()
    }

    /// The current name of a workspace.
    pub fn workspace_name(&self, workspace_id: Uuid) -> String {
        let mut conn = self.db.pool().get().unwrap();
        workspaces::table
            .find(workspace_id)
            .select(workspaces::name)
            .first(&mut conn)
            .unwrap()
    }

    /// Grants `key` directly, globally or within `scope`.
    pub fn grant(&self, user: UserId, key: &str, scope: Option<ResourceScope>) {
        self.client
            .grant(None, &user.to_string(), PermissionKey::new(key), scope)
            .unwrap();
    }

    /// Creates a role with a unique name and the given permission keys.
    pub fn role(&self, keys: &[&str]) -> air_tight::RoleId {
        let name = RoleName::new(&format!("role-{}", Uuid::new_v4())).unwrap();
        let role = self.client.create_role(None, &name, None).unwrap();
        self.set_role(role, keys);
        role
    }

    /// Replaces a role's permission keys.
    pub fn set_role(&self, role: air_tight::RoleId, keys: &[&str]) {
        let keys: Vec<PermissionKey> = keys.iter().map(|k| PermissionKey::new(*k)).collect();
        self.client.set_role_permissions(None, role, &keys).unwrap();
    }

    /// Sends a request and returns the status and the JSON (or `Null`) body.
    pub async fn send(
        &self,
        method: &str,
        path: &str,
        user: Option<UserId>,
        body: Option<Value>,
    ) -> (StatusCode, Value) {
        let mut req = Request::builder().method(method).uri(path);
        if let Some(user) = user {
            req = req.header("x-user-id", user.to_string());
        }
        let req = match body {
            Some(json) => req
                .header("content-type", "application/json")
                .body(Body::from(json.to_string())),
            None => req.body(Body::empty()),
        }
        .unwrap();
        let res = self.app.clone().oneshot(req).await.unwrap();
        let status = res.status();
        let bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
            .await
            .unwrap();
        let json = serde_json::from_slice(&bytes)
            .unwrap_or_else(|_| Value::String(String::from_utf8_lossy(&bytes).into_owned()));
        (status, json)
    }

    /// `GET` as `user`, returning only the status.
    pub async fn get(&self, path: &str, user: UserId) -> StatusCode {
        self.send("GET", path, Some(user), None).await.0
    }
}

/// A workspace scope.
pub fn ws(id: Uuid) -> Option<ResourceScope> {
    Some(ResourceScope::new(
        MyResources::Workspace.key(),
        id.to_string(),
    ))
}

/// An invoice scope.
pub fn inv(id: Uuid) -> Option<ResourceScope> {
    Some(ResourceScope::new(
        ResourceKindKey::new("Invoice"),
        id.to_string(),
    ))
}

/// The ids of the objects in a JSON list response.
pub fn ids(list: &Value) -> Vec<String> {
    let mut ids: Vec<String> = list
        .as_array()
        .expect("a JSON array")
        .iter()
        .map(|row| row["id"].as_str().unwrap().to_owned())
        .collect();
    ids.sort();
    ids
}
