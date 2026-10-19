use serde::Deserialize;

use super::ScopeDto;

/// Body of `POST /admin/grant`.
#[derive(Debug, Deserialize)]
pub struct GrantPermission {
    /// The user receiving the grant.
    pub user_id: String,
    /// The permission key, for example `Invoice::Read` or `*`.
    pub permission: String,
    /// Limits the grant to one resource; global when absent.
    #[serde(default)]
    pub scope: Option<ScopeDto>,
}
