use serde::Deserialize;

/// Body of `POST /admin/roles`.
#[derive(Debug, Deserialize)]
pub struct CreateRole {
    /// The unique role name.
    pub name: String,
    /// Optional description.
    #[serde(default)]
    pub description: Option<String>,
    /// Permission keys of the role, for example `Workspace::Read` or `Billing::*`.
    #[serde(default)]
    pub permissions: Vec<String>,
}
