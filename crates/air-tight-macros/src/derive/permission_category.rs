//! `#[derive(PermissionCategory)]`: the top-level permission enum, one tuple
//! variant per category wrapping a `Permission` enum.

use std::collections::HashMap;

use proc_macro2::TokenStream;
use quote::{quote, quote_spanned};
use syn::spanned::Spanned;
use syn::{DeriveInput, Fields, Ident, Path, Type};

use crate::attrs::{self, Allowed};
use crate::util;

const DERIVE: &str = "PermissionCategory";

struct Category<'a> {
    ident: &'a Ident,
    name: String,
    action_ty: &'a Type,
    resource: Option<Path>,
}

/// Expands the derive.
pub(crate) fn expand(input: &DeriveInput) -> syn::Result<TokenStream> {
    util::no_generics(input, DERIVE)?;
    let data = util::expect_enum(input, DERIVE)?;
    attrs::forbid(&input.attrs, "the enum itself; put them on variants")?;
    if data.variants.is_empty() {
        return Err(syn::Error::new_spanned(
            &input.ident,
            "#[derive(PermissionCategory)] requires at least one category variant",
        ));
    }

    let allowed = Allowed {
        key: true,
        resource: true,
        nested: false,
    };
    let mut cats = Vec::new();
    let mut seen: HashMap<String, Ident> = HashMap::new();
    for v in &data.variants {
        let action_ty = match &v.fields {
            Fields::Unnamed(f) if f.unnamed.len() == 1 => &f.unnamed[0].ty,
            _ => {
                return Err(syn::Error::new_spanned(
                    v,
                    format!(
                        "#[derive(PermissionCategory)] requires tuple variants with exactly one \
                         field whose type derives Permission, e.g. `{}(MyAction)`",
                        v.ident
                    ),
                ));
            }
        };
        let a = attrs::parse(&v.attrs, allowed)?;
        let name = a.key.map_or_else(|| v.ident.to_string(), |k| k.value());
        if let Some(prev) = seen.insert(name.clone(), v.ident.clone()) {
            return Err(syn::Error::new_spanned(
                &v.ident,
                format!("duplicate category key `{name}` (also used by `{prev}`)"),
            ));
        }
        cats.push(Category {
            ident: &v.ident,
            name,
            action_ty,
            resource: a.resource,
        });
    }

    let name = &input.ident;
    let idents: Vec<_> = cats.iter().map(|c| c.ident).collect();
    let names: Vec<_> = cats.iter().map(|c| c.name.as_str()).collect();
    let tys: Vec<_> = cats.iter().map(|c| c.action_ty).collect();
    let variant_names: Vec<String> = idents.iter().map(|i| i.to_string()).collect();

    let asserts = tys.iter().map(|ty| {
        quote_spanned! {ty.span()=>
            const _: fn() = || {
                fn __air_tight_assert_permission<T: ::air_tight::Permission>() {}
                __air_tight_assert_permission::<#ty>();
            };
        }
    });
    let resource_arms = cats.iter().map(|c| {
        let ident = c.ident;
        match &c.resource {
            Some(path) => {
                let key = quote_spanned! {path.span()=>
                    <#path as ::air_tight::ResourceKindMarker>::key()
                };
                quote! { Self::#ident(_) => ::core::option::Option::Some(#key), }
            }
            None => quote! { Self::#ident(_) => ::core::option::Option::None, },
        }
    });

    Ok(quote! {
        #(#asserts)*

        impl ::core::clone::Clone for #name {
            fn clone(&self) -> Self { *self }
        }
        impl ::core::marker::Copy for #name {}
        impl ::core::cmp::PartialEq for #name {
            #[allow(unreachable_patterns)]
            fn eq(&self, other: &Self) -> bool {
                match (self, other) {
                    #((Self::#idents(a), Self::#idents(b)) => a == b,)*
                    _ => false,
                }
            }
        }
        impl ::core::cmp::Eq for #name {}
        impl ::core::hash::Hash for #name {
            fn hash<H: ::core::hash::Hasher>(&self, state: &mut H) {
                ::core::hash::Hash::hash(&::core::mem::discriminant(self), state);
                match self { #(Self::#idents(a) => ::core::hash::Hash::hash(a, state),)* }
            }
        }
        impl ::core::fmt::Debug for #name {
            fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                match self {
                    #(Self::#idents(a) => f.debug_tuple(stringify!(#idents)).field(a).finish(),)*
                }
            }
        }

        impl #name {
            /// Looks up a permission by Rust variant names (category, action)
            /// in `const` context. Generated for `#[air_tight::permission(...)]`;
            /// not public API.
            #[doc(hidden)]
            pub const fn __air_tight_lookup(
                category: &str,
                action: &str,
            ) -> ::core::option::Option<Self> {
                #(
                    if ::air_tight::const_str_eq(category, #variant_names) {
                        return match <#tys>::__air_tight_variant(action) {
                            ::core::option::Option::Some(a) => ::core::option::Option::Some(Self::#idents(a)),
                            ::core::option::Option::None => ::core::option::Option::None,
                        };
                    }
                )*
                ::core::option::Option::None
            }
        }

        impl ::air_tight::PermissionCategory for #name {
            fn key(&self) -> ::air_tight::PermissionKey {
                static KEYS: ::std::sync::OnceLock<
                    ::std::collections::HashMap<#name, &'static str>,
                > = ::std::sync::OnceLock::new();
                let keys = KEYS.get_or_init(|| {
                    <Self as ::air_tight::PermissionCategory>::all()
                        .iter()
                        .map(|p| {
                            let action = match p {
                                #(Self::#idents(a) => ::air_tight::Permission::action_name(a),)*
                            };
                            let cat = ::air_tight::PermissionCategory::category_name(p);
                            let key: &'static str = ::std::boxed::Box::leak(
                                ::std::format!("{cat}::{action}").into_boxed_str(),
                            );
                            (*p, key)
                        })
                        .collect()
                });
                ::air_tight::PermissionKey::from_static(keys[self])
            }

            fn category_name(&self) -> &'static str {
                match self { #(Self::#idents(_) => #names,)* }
            }

            fn all() -> &'static [Self] {
                static ALL: ::std::sync::OnceLock<::std::vec::Vec<#name>> =
                    ::std::sync::OnceLock::new();
                ALL.get_or_init(|| {
                    let mut v = ::std::vec::Vec::new();
                    #(
                        v.extend(
                            <#tys as ::air_tight::Permission>::all()
                                .iter()
                                .map(|a| Self::#idents(*a)),
                        );
                    )*
                    v
                })
            }

            fn resource_kind(&self) -> ::core::option::Option<::air_tight::ResourceKindKey> {
                match self { #(#resource_arms)* }
            }

            fn from_key(key: &str) -> ::core::option::Option<Self> {
                let (cat, action) = key.split_once("::")?;
                match cat {
                    #(
                        #names => <#tys as ::air_tight::Permission>::all()
                            .iter()
                            .find(|a| ::air_tight::Permission::action_name(*a) == action)
                            .map(|a| Self::#idents(*a)),
                    )*
                    _ => ::core::option::Option::None,
                }
            }
        }
    })
}
