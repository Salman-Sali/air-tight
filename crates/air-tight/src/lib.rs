//! Runtime crate for air-tight. Re-exports the core types and the macros.
#![deny(missing_docs)]

mod error;

pub use air_tight_core::*;
pub use air_tight_macros::{IdentifyResources, Permission, PermissionCategory, ResourceKind};
pub use error::Error;
