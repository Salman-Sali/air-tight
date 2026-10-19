use serde::{Deserialize, Serialize};

use crate::ResourceKindKey;

/// A specific resource, identified by kind and id.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ResourceScope {
    /// The kind of the resource.
    pub kind: ResourceKindKey,
    /// The id of the resource, stored as text.
    pub id: String,
}

impl ResourceScope {
    /// Creates a scope from a kind and any string-like id.
    pub fn new(kind: ResourceKindKey, id: impl Into<String>) -> Self {
        ResourceScope {
            kind,
            id: id.into(),
        }
    }
}
