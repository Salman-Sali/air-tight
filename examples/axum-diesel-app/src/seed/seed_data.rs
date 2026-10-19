use air_tight::{PermissionKey, ResourceKind, ResourceScope, RoleId, RoleName};
use diesel::prelude::*;
use uuid::Uuid;

use super::ids::*;
use crate::authz::MyResources;
use crate::error::ApiError;
use crate::models::{NewInvoice, UserRecord, WorkspaceRecord};
use crate::schema::{invoices, users, workspaces};
use crate::state::AppState;

/// Inserts the demo users, workspaces, invoices and grants.
///
/// Safe to run on every start: rows use fixed ids with `ON CONFLICT DO NOTHING`,
/// the `Member` role is created only if absent, and the grants are idempotent.
///
/// - alice: the global wildcard `*`
/// - bob: role `Member` (`Workspace::Read`, `Invoice::Read`) scoped to workspace A
/// - carol: nothing
pub fn seed(state: &AppState) -> Result<(), ApiError> {
    let mut conn = state.db.pool().get()?;
    seed_rows(&mut conn)?;

    let client = &state.air_tight;
    let actor = Some("seed");

    client.grant(actor, &ALICE.to_string(), PermissionKey::new("*"), None)?;

    let member = RoleName::new(MEMBER_ROLE)?;
    let role = match client
        .list_roles()?
        .into_iter()
        .find(|r| r.name == MEMBER_ROLE)
    {
        Some(existing) => existing.id,
        None => match client.create_role(actor, &member, Some("Read access within a workspace")) {
            Ok(id) => id,
            // Lost a race with another process seeding at the same time.
            Err(_) => find_role(state)?,
        },
    };
    client.set_role_permissions(
        actor,
        role,
        &[
            PermissionKey::new("Workspace::Read"),
            PermissionKey::new("Invoice::Read"),
        ],
    )?;
    client.assign_role(
        actor,
        &BOB.to_string(),
        role,
        Some(ResourceScope::new(
            MyResources::Workspace.key(),
            WORKSPACE_A.to_string(),
        )),
    )?;
    Ok(())
}

fn find_role(state: &AppState) -> Result<RoleId, ApiError> {
    state
        .air_tight
        .list_roles()?
        .into_iter()
        .find(|r| r.name == MEMBER_ROLE)
        .map(|r| r.id)
        .ok_or(ApiError::Internal)
}

fn seed_rows(conn: &mut PgConnection) -> Result<(), ApiError> {
    let people = [(ALICE, "alice"), (BOB, "bob"), (CAROL, "carol")];
    let people: Vec<UserRecord> = people
        .into_iter()
        .map(|(id, name)| UserRecord {
            id,
            name: name.to_owned(),
        })
        .collect();
    diesel::insert_into(users::table)
        .values(&people)
        .on_conflict_do_nothing()
        .execute(conn)?;

    let spaces = vec![
        WorkspaceRecord {
            id: WORKSPACE_A,
            name: "Workspace A".to_owned(),
        },
        WorkspaceRecord {
            id: WORKSPACE_B,
            name: "Workspace B".to_owned(),
        },
    ];
    diesel::insert_into(workspaces::table)
        .values(&spaces)
        .on_conflict_do_nothing()
        .execute(conn)?;

    let bills: Vec<NewInvoice> = [
        (INVOICE_A, WORKSPACE_A, 1_000),
        (INVOICE_B, WORKSPACE_B, 2_000),
    ]
    .into_iter()
    .map(|(id, workspace_id, amount): (Uuid, Uuid, i64)| NewInvoice {
        id,
        workspace_id,
        amount,
    })
    .collect();
    diesel::insert_into(invoices::table)
        .values(&bills)
        .on_conflict_do_nothing()
        .execute(conn)?;
    Ok(())
}
