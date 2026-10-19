use air_tight::AirTightPath;
use axum::Json;
use axum::extract::State;
use diesel::prelude::*;
use uuid::Uuid;

use crate::authz::my_resources_kinds::Workspace;
use crate::db::Db;
use crate::error::ApiError;
use crate::models::WorkspaceRecord;
use crate::schema::workspaces;

/// `GET /workspaces/{id}`: needs `Workspace(Read)` on that workspace.
#[air_tight::permission(Workspace(Read))]
pub async fn get_workspace(
    State(db): State<Db>,
    id: AirTightPath<Uuid, Workspace>,
) -> Result<Json<WorkspaceRecord>, ApiError> {
    let id = id.into_inner();
    let row = db
        .run(move |conn| {
            Ok(workspaces::table
                .find(id)
                .select(WorkspaceRecord::as_select())
                .first(conn)?)
        })
        .await?;
    Ok(Json(row))
}
