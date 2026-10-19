use air_tight::{ResourceKind, ResourceKindKey, ResourceScope};
use serde::Deserialize;

use crate::authz::MyResources;
use crate::error::ApiError;

/// A resource scope as written in admin request bodies.
#[derive(Clone, Debug, Deserialize)]
pub struct ScopeDto {
    /// The resource kind key, for example `Workspace`.
    pub kind: String,
    /// The resource id.
    pub id: String,
}

impl ScopeDto {
    /// Validates the kind and converts to a [`ResourceScope`].
    pub fn into_scope(self) -> Result<ResourceScope, ApiError> {
        if MyResources::from_key(&self.kind).is_none() {
            return Err(ApiError::BadRequest(format!(
                "unknown resource kind `{}`",
                self.kind
            )));
        }
        Ok(ResourceScope::new(ResourceKindKey::new(self.kind), self.id))
    }
}
