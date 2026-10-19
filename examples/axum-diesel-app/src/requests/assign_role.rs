use serde::Deserialize;
use uuid::Uuid;

use super::ScopeDto;

/// Body of `POST /admin/assign`.
#[derive(Debug, Deserialize)]
pub struct AssignRole {
    /// The user receiving the role.
    pub user_id: String,
    /// The role to assign.
    pub role_id: Uuid,
    /// Limits the role to one resource; global when absent.
    #[serde(default)]
    pub scope: Option<ScopeDto>,
}
