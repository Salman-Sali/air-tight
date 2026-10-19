use air_tight::{Access, CurrentUser, ResourceKind};
use axum::Json;
use axum::extract::State;
use diesel::prelude::*;

use crate::auth::AuthenticatedUser;
use crate::authz::{MyPerms, MyResources, WorkspaceAction};
use crate::db::{Db, blocking};
use crate::error::ApiError;
use crate::models::WorkspaceRecord;
use crate::schema::workspaces;
use crate::state::Client;

/// `GET /workspaces`: the workspaces the caller may read, filtered with
/// `accessible_ids` (served from the grant cache).
///
/// Any authenticated user may call it; what they see depends on their grants,
/// so the handler is `unrestricted` rather than guarded by one permission.
#[air_tight::unrestricted]
pub async fn list_workspaces(
    State(client): State<Client>,
    State(db): State<Db>,
    CurrentUser(user): CurrentUser<AuthenticatedUser>,
) -> Result<Json<Vec<WorkspaceRecord>>, ApiError> {
    let user_id = user.id.to_string();
    let access = blocking(move || {
        Ok(client.accessible_ids(
            &user_id,
            MyResources::Workspace.key(),
            MyPerms::Workspace(WorkspaceAction::Read),
        )?)
    })
    .await?;
    let rows = db
        .run(move |conn| {
            let query = workspaces::table
                .select(WorkspaceRecord::as_select())
                .order(workspaces::name)
                .into_boxed();
            Ok(match access {
                Access::All => query.load(conn)?,
                Access::Some(ids) => {
                    let ids: Vec<uuid::Uuid> = ids
                        .iter()
                        .filter_map(|id| uuid::Uuid::parse_str(id).ok())
                        .collect();
                    query.filter(workspaces::id.eq_any(ids)).load(conn)?
                }
                Access::None => Vec::new(),
            })
        })
        .await?;
    Ok(Json(rows))
}
