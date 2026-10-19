use serde::Deserialize;

/// Body of `PATCH /workspaces/{id}`.
#[derive(Debug, Deserialize)]
pub struct RenameWorkspace {
    /// The new name.
    pub name: String,
}
