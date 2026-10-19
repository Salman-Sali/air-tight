use air_tight::Permission;

/// Actions on a workspace.
#[derive(Permission)]
pub enum WorkspaceAction {
    /// View a workspace.
    Read,
    /// Rename or edit a workspace.
    Update,
    /// Delete a workspace.
    Delete,
}
