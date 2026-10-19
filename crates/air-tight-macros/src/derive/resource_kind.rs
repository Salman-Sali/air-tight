//! `#[derive(ResourceKind)]`: the app's resource kinds plus marker types.

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{DeriveInput, Fields};

use crate::attrs;
use crate::util;

const DERIVE: &str = "ResourceKind";

/// Expands the derive.
pub(crate) fn expand(input: &DeriveInput) -> syn::Result<TokenStream> {
    util::no_generics(input, DERIVE)?;
    let data = util::expect_enum(input, DERIVE)?;
    attrs::forbid(&input.attrs, "#[derive(ResourceKind)] enums")?;
    if data.variants.is_empty() {
        return Err(syn::Error::new_spanned(
            &input.ident,
            "#[derive(ResourceKind)] requires at least one variant",
        ));
    }
    let mut idents = Vec::new();
    for v in &data.variants {
        if !matches!(v.fields, Fields::Unit) {
            return Err(syn::Error::new_spanned(
                v,
                format!(
                    "#[derive(ResourceKind)] supports unit variants only; `{}` has fields",
                    v.ident
                ),
            ));
        }
        attrs::forbid(
            &v.attrs,
            "resource kind variants (the key is the variant name)",
        )?;
        idents.push(&v.ident);
    }
    let keys: Vec<String> = idents.iter().map(|i| i.to_string()).collect();

    let name = &input.ident;
    let vis = &input.vis;
    let module = format_ident!("{}_kinds", util::snake_case(&name.to_string()));
    let std_impls = util::unit_enum_std_impls(name, &idents);
    let mod_doc = format!("Marker types for each variant of [`{name}`], one unit struct per kind.");
    let marker_docs = idents
        .iter()
        .map(|i| format!("Marker type for the `{i}` resource kind."));

    Ok(quote! {
        #std_impls
        impl ::air_tight::ResourceKind for #name {
            fn key(&self) -> ::air_tight::ResourceKindKey {
                match self {
                    #(Self::#idents => ::air_tight::ResourceKindKey::from_static(#keys),)*
                }
            }
            fn all() -> &'static [Self] {
                &[#(Self::#idents),*]
            }
            fn from_key(key: &str) -> ::core::option::Option<Self> {
                match key {
                    #(#keys => ::core::option::Option::Some(Self::#idents),)*
                    _ => ::core::option::Option::None,
                }
            }
        }

        #[doc = #mod_doc]
        #vis mod #module {
            #(
                #[doc = #marker_docs]
                #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
                pub struct #idents;

                impl ::air_tight::ResourceKindMarker for #idents {
                    fn key() -> ::air_tight::ResourceKindKey {
                        ::air_tight::ResourceKindKey::from_static(#keys)
                    }
                }

                impl ::core::convert::From<#idents> for super::#name {
                    fn from(_: #idents) -> Self { super::#name::#idents }
                }
            )*
        }
    })
}
