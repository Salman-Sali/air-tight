//! Small helpers shared by the derives.

use proc_macro2::TokenStream;
use quote::quote;
use syn::{DeriveInput, Ident};

/// Rejects generic parameters on enums that must be plain catalogs.
pub(crate) fn no_generics(input: &DeriveInput, derive: &str) -> syn::Result<()> {
    if input.generics.params.is_empty() {
        Ok(())
    } else {
        Err(syn::Error::new_spanned(
            &input.generics,
            format!("#[derive({derive})] does not support generic parameters"),
        ))
    }
}

/// Returns the enum data or an error spanned on the item.
pub(crate) fn expect_enum<'a>(
    input: &'a DeriveInput,
    derive: &str,
) -> syn::Result<&'a syn::DataEnum> {
    match &input.data {
        syn::Data::Enum(e) => Ok(e),
        _ => Err(syn::Error::new_spanned(
            &input.ident,
            format!("#[derive({derive})] can only be used on enums"),
        )),
    }
}

/// `CamelCase` to `snake_case`.
pub(crate) fn snake_case(s: &str) -> String {
    let mut out = String::new();
    let chars: Vec<char> = s.chars().collect();
    for (i, c) in chars.iter().enumerate() {
        if c.is_uppercase() {
            let prev_lower = i > 0 && !chars[i - 1].is_uppercase() && chars[i - 1] != '_';
            let next_lower = chars.get(i + 1).is_some_and(|n| n.is_lowercase());
            let prev_upper = i > 0 && chars[i - 1].is_uppercase();
            if prev_lower || (prev_upper && next_lower) {
                out.push('_');
            }
            out.extend(c.to_lowercase());
        } else {
            out.push(*c);
        }
    }
    out
}

/// Emits `Clone`, `Copy`, `PartialEq`, `Eq`, `Hash` and `Debug` for a
/// fieldless enum. The derives generate these so the catalog traits'
/// supertraits are always satisfied.
pub(crate) fn unit_enum_std_impls(name: &Ident, variants: &[&Ident]) -> TokenStream {
    let names = variants.iter().map(|v| v.to_string());
    quote! {
        impl ::core::clone::Clone for #name {
            fn clone(&self) -> Self { *self }
        }
        impl ::core::marker::Copy for #name {}
        impl ::core::cmp::PartialEq for #name {
            fn eq(&self, other: &Self) -> bool {
                ::core::mem::discriminant(self) == ::core::mem::discriminant(other)
            }
        }
        impl ::core::cmp::Eq for #name {}
        impl ::core::hash::Hash for #name {
            fn hash<H: ::core::hash::Hasher>(&self, state: &mut H) {
                ::core::hash::Hash::hash(&::core::mem::discriminant(self), state);
            }
        }
        impl ::core::fmt::Debug for #name {
            fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                f.write_str(match self { #(Self::#variants => #names,)* })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::snake_case;

    #[test]
    fn snake() {
        assert_eq!(snake_case("MyResources"), "my_resources");
        assert_eq!(snake_case("Resources"), "resources");
        assert_eq!(snake_case("HTTPKinds"), "http_kinds");
        assert_eq!(snake_case("A"), "a");
    }
}
