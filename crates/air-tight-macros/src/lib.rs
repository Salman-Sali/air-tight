//! Procedural macros for air-tight.
//!
//! Use these through the `air_tight` crate, which re-exports them. Generated
//! code refers to `::air_tight::...`, so the `air-tight` crate must be a
//! dependency of the crate using the macros.
#![deny(missing_docs)]

use proc_macro::TokenStream;
use syn::{DeriveInput, parse_macro_input};

mod attrs;
mod derive;
mod util;

fn finish(result: syn::Result<proc_macro2::TokenStream>) -> TokenStream {
    result.unwrap_or_else(syn::Error::into_compile_error).into()
}

/// Derives `air_tight::Permission` for an enum of unit action variants.
///
/// Also implements `Clone`, `Copy`, `PartialEq`, `Eq`, `Hash` and `Debug`, so
/// do not derive those yourself. `#[air_tight(key = "...")]` on a variant pins
/// its key segment (useful after a rename).
#[proc_macro_derive(Permission, attributes(air_tight))]
pub fn derive_permission(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    finish(derive::permission::expand(&input))
}

/// Derives `air_tight::PermissionCategory` for an enum whose variants each
/// wrap exactly one `Permission` enum.
///
/// Also implements `Clone`, `Copy`, `PartialEq`, `Eq`, `Hash` and `Debug`.
/// Variant attributes: `#[air_tight(resource = Marker)]` binds the category
/// to a resource kind marker type, `#[air_tight(key = "...")]` pins the
/// category segment of its keys.
#[proc_macro_derive(PermissionCategory, attributes(air_tight))]
pub fn derive_permission_category(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    finish(derive::permission_category::expand(&input))
}

/// Derives `air_tight::ResourceKind` for an enum of unit variants.
///
/// Also implements `Clone`, `Copy`, `PartialEq`, `Eq`, `Hash` and `Debug`,
/// and emits a module `<snake_case_enum_name>_kinds` holding one marker unit
/// struct per variant (e.g. `my_resources_kinds::Workspace`), each
/// implementing `ResourceKindMarker`.
#[proc_macro_derive(ResourceKind, attributes(air_tight))]
pub fn derive_resource_kind(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    finish(derive::resource_kind::expand(&input))
}

/// Derives `air_tight::IdentifyResources` for a struct or enum.
///
/// Only fields marked `#[air_tight(resource = Marker)]` (ids via
/// `ResourceIds`) or `#[air_tight(nested)]` (via `IdentifyResources`)
/// contribute scopes; nothing is guessed.
#[proc_macro_derive(IdentifyResources, attributes(air_tight))]
pub fn derive_identify_resources(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    finish(derive::identify_resources::expand(&input))
}
