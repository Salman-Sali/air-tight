//! `#[derive(Permission)]`: a layer of action variants.

use std::collections::HashMap;

use proc_macro2::TokenStream;
use quote::quote;
use syn::{DeriveInput, Fields};

use crate::attrs::{self, Allowed};
use crate::util;

const DERIVE: &str = "Permission";

/// Expands the derive.
pub(crate) fn expand(input: &DeriveInput) -> syn::Result<TokenStream> {
    util::no_generics(input, DERIVE)?;
    let data = util::expect_enum(input, DERIVE)?;
    attrs::forbid(&input.attrs, "the enum itself; put them on variants")?;
    if data.variants.is_empty() {
        return Err(syn::Error::new_spanned(
            &input.ident,
            "#[derive(Permission)] requires at least one action variant",
        ));
    }

    let allowed = Allowed {
        key: true,
        resource: false,
        nested: false,
    };
    let mut idents = Vec::new();
    let mut names = Vec::new();
    let mut seen: HashMap<String, syn::Ident> = HashMap::new();
    for v in &data.variants {
        if !matches!(v.fields, Fields::Unit) {
            return Err(syn::Error::new_spanned(
                v,
                format!(
                    "#[derive(Permission)] supports unit variants only; `{}` has fields",
                    v.ident
                ),
            ));
        }
        let a = attrs::parse(&v.attrs, allowed)?;
        let name = a.key.map_or_else(|| v.ident.to_string(), |k| k.value());
        if let Some(prev) = seen.insert(name.clone(), v.ident.clone()) {
            return Err(syn::Error::new_spanned(
                &v.ident,
                format!("duplicate action key `{name}` (also used by `{prev}`)"),
            ));
        }
        idents.push(&v.ident);
        names.push(name);
    }

    let name = &input.ident;
    let std_impls = util::unit_enum_std_impls(name, &idents);
    let variant_names: Vec<String> = idents.iter().map(|i| i.to_string()).collect();
    Ok(quote! {
        #std_impls
        impl #name {
            /// Looks up an action by its Rust variant name in `const` context.
            /// Generated for `#[air_tight::permission(...)]`; not public API.
            #[doc(hidden)]
            pub const fn __air_tight_variant(name: &str) -> ::core::option::Option<Self> {
                #(
                    if ::air_tight::const_str_eq(name, #variant_names) {
                        return ::core::option::Option::Some(Self::#idents);
                    }
                )*
                ::core::option::Option::None
            }
        }
        impl ::air_tight::Permission for #name {
            fn action_name(&self) -> &'static str {
                match self { #(Self::#idents => #names,)* }
            }
            fn all() -> &'static [Self] {
                &[#(Self::#idents),*]
            }
        }
    })
}
