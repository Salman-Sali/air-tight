use diesel::prelude::*;
use serde::Serialize;
use uuid::Uuid;

use crate::schema::workspaces;

/// A row of `workspaces`.
#[derive(Clone, Debug, Queryable, Selectable, Insertable, Serialize)]
#[diesel(table_name = workspaces)]
pub struct WorkspaceRecord {
    /// The workspace id.
    pub id: Uuid,
    /// The workspace name.
    pub name: String,
}
