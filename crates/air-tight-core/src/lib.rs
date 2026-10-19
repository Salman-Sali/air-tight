//! Core traits, types and the condition tree for air-tight.
//!
//! This crate has no dependency on axum, diesel or moka. It holds the traits
//! the derives implement ([`Permission`], [`PermissionCategory`],
//! [`ResourceKind`], [`IdentifyResources`], [`Principal`]), the key and scope
//! newtypes ([`PermissionKey`], [`ResourceKindKey`], [`ResourceScope`]) and the
//! [`Condition`] tree with wildcard matching.
//!
//! You normally do not depend on this crate directly: use the `air_tight`
//! crate, which re-exports everything here together with the runtime and the
//! macros. See its crate documentation for the walkthrough.
#![deny(missing_docs)]

mod condition;
mod const_str_eq;
mod identify_resources;
mod permission;
mod permission_category;
mod permission_key;
mod principal;
mod resource_ids;
mod resource_kind;
mod resource_kind_key;
mod resource_kind_marker;
mod resource_kinds;
mod resource_scope;
mod wildcard;

#[cfg(test)]
mod test_support;

pub use condition::{Condition, Leaf};
#[doc(hidden)]
pub use const_str_eq::const_str_eq;
pub use identify_resources::IdentifyResources;
pub use permission::Permission;
pub use permission_category::PermissionCategory;
pub use permission_key::PermissionKey;
pub use principal::Principal;
pub use resource_ids::ResourceIds;
pub use resource_kind::ResourceKind;
pub use resource_kind_key::ResourceKindKey;
pub use resource_kind_marker::ResourceKindMarker;
pub use resource_kinds::ResourceKinds;
pub use resource_scope::ResourceScope;
pub use wildcard::{grant_matches, is_well_formed_wildcard};
