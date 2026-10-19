use air_tight::{CurrentUser, ResourceKind};
use axum::Json;
use axum::extract::State;
use diesel::prelude::*;

use crate::auth::AuthenticatedUser;
use crate::authz::{MyPerms, MyResources, WorkspaceAction};
use crate::db::Db;
use crate::error::ApiError;
use crate::models::WorkspaceRecord;
use crate::schema::workspaces;
use crate::state::Client;

/// `GET /workspaces/sql`: the same list as `GET /workspaces`, filtered inside
/// the SQL query with the diesel `accessible` expression (reads the grant
/// tables directly, bypassing the cache).
#[air_tight::unrestricted]
pub async fn list_workspaces_sql(
    State(client): State<Client>,
    State(db): State<Db>,
    CurrentUser(user): CurrentUser<AuthenticatedUser>,
) -> Result<Json<Vec<WorkspaceRecord>>, ApiError> {
    let user_id = user.id.to_string();
    let rows = db
        .run(move |conn| {
            Ok(workspaces::table
                .filter(client.accessible(
                    &user_id,
                    MyResources::Workspace.key(),
                    MyPerms::Workspace(WorkspaceAction::Read),
                    workspaces::id,
                ))
                .select(WorkspaceRecord::as_select())
                .order(workspaces::name)
                .load(conn)?)
        })
        .await?;
    Ok(Json(rows))
}
