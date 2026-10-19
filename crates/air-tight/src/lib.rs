//! Runtime crate for air-tight. Re-exports the core types.
#![deny(missing_docs)]

mod error;

pub use air_tight_core::*;
pub use error::Error;
