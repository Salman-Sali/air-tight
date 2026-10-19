//! Core traits, types and the condition tree for air-tight.
//!
//! This crate has no dependency on axum, diesel or moka.
#![deny(missing_docs)]

mod condition;
mod identify_resources;
mod permission;
mod permission_category;
mod permission_key;
mod principal;
mod resource_ids;
mod resource_kind;
mod resource_kind_key;
mod resource_kind_marker;
mod resource_scope;
mod wildcard;

#[cfg(test)]
mod test_support;

pub use condition::{Condition, Leaf};
pub use identify_resources::IdentifyResources;
pub use permission::Permission;
pub use permission_category::PermissionCategory;
pub use permission_key::PermissionKey;
pub use principal::Principal;
pub use resource_ids::ResourceIds;
pub use resource_kind::ResourceKind;
pub use resource_kind_key::ResourceKindKey;
pub use resource_kind_marker::ResourceKindMarker;
pub use resource_scope::ResourceScope;
pub use wildcard::{grant_matches, is_well_formed_wildcard};
