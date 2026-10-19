use diesel::prelude::*;

use crate::db::Db;
use crate::error::ApiError;
use crate::models::WorkspaceRecord;
use crate::requests::WorkspaceRef;
use crate::schema::workspaces;

/// Renames a workspace.
///
/// A `kind = fn` service: the macro prepends `(&AirTight, &User,
/// Vec<ResourceScope>)` to the parameters and checks `Workspace(Update)`
/// against the scopes of `workspace` (plus any the caller passes) before the
/// body runs, so the rule travels with the function wherever it is called from.
#[air_tight::permission(Workspace(Update), kind = fn, resources = [workspace])]
pub async fn rename_workspace(
    db: &Db,
    workspace: WorkspaceRef,
    name: String,
) -> Result<WorkspaceRecord, ApiError> {
    let id = workspace.id;
    db.run(move |conn| {
        Ok(diesel::update(workspaces::table.find(id))
            .set(workspaces::name.eq(name))
            .returning(WorkspaceRecord::as_returning())
            .get_result(conn)?)
    })
    .await
}
