use std::fmt;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Identifier of a role.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RoleId(Uuid);

impl RoleId {
    /// Generates a new random role id.
    pub fn new() -> Self {
        RoleId(Uuid::new_v4())
    }

    /// Wraps an existing uuid.
    pub const fn from_uuid(id: Uuid) -> Self {
        RoleId(id)
    }

    /// Returns the underlying uuid.
    pub const fn as_uuid(&self) -> Uuid {
        self.0
    }
}

impl Default for RoleId {
    fn default() -> Self {
        Self::new()
    }
}

impl From<Uuid> for RoleId {
    fn from(id: Uuid) -> Self {
        RoleId(id)
    }
}

impl From<RoleId> for Uuid {
    fn from(id: RoleId) -> Self {
        id.0
    }
}

impl fmt::Display for RoleId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}
