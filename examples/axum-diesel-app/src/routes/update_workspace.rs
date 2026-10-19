use air_tight::{AirTightPath, CurrentUser};
use axum::Json;
use axum::extract::State;
use uuid::Uuid;

use crate::auth::AuthenticatedUser;
use crate::authz::my_resources_kinds::Workspace;
use crate::db::Db;
use crate::error::ApiError;
use crate::models::WorkspaceRecord;
use crate::requests::{RenameWorkspace, WorkspaceRef};
use crate::services;
use crate::state::Client;

/// `PATCH /workspaces/{id}`: needs `Workspace(Update)` on that workspace.
///
/// The work is done by [`services::rename_workspace`], a `kind = fn` service
/// that checks the same permission itself. The handler binds `state` and
/// `user` so it can hand the client and the caller to that service.
#[air_tight::permission(Workspace(Update), state = client, user = me)]
pub async fn update_workspace(
    State(client): State<Client>,
    CurrentUser(me): CurrentUser<AuthenticatedUser>,
    State(db): State<Db>,
    id: AirTightPath<Uuid, Workspace>,
    Json(body): Json<RenameWorkspace>,
) -> Result<Json<WorkspaceRecord>, ApiError> {
    let workspace = WorkspaceRef {
        id: id.into_inner(),
    };
    let row =
        services::rename_workspace(&client, &me, Vec::new(), &db, workspace, body.name).await?;
    Ok(Json(row))
}
