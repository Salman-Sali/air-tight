//! `#[air_tight::unrestricted]`: a public handler for the typed router.

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::ItemFn;

use super::factory::{self, Factory, Wrapper};
use super::params;

/// Expands a public handler.
pub(crate) fn expand(args: TokenStream, func: ItemFn) -> syn::Result<TokenStream> {
    if !args.is_empty() {
        return Err(syn::Error::new_spanned(
            args,
            "#[air_tight::unrestricted] takes no arguments",
        ));
    }
    let ItemFn {
        attrs,
        vis,
        mut sig,
        block,
    } = func;
    if sig.asyncness.is_none() {
        return Err(syn::Error::new(
            sig.fn_token.span,
            "#[air_tight::unrestricted] handlers must be `async fn`",
        ));
    }
    if !sig.generics.params.is_empty() {
        return Err(syn::Error::new_spanned(
            &sig.generics,
            "#[air_tight::unrestricted] handlers cannot be generic",
        ));
    }
    let typed = params::typed(sig.inputs.clone())?;
    let name = sig.ident.clone();
    let inner = format_ident!("__air_tight_handler_{}", name);
    sig.ident = inner.clone();
    let (doc_attrs, other_attrs) = factory::split_docs(attrs);
    let factory = factory::emit(&Factory {
        wrapper: Wrapper::Public,
        vis: &vis,
        name: &name,
        inner: &inner,
        doc_attrs: &doc_attrs,
        access_doc: "Public: no permission required.".to_owned(),
        param_tys: typed.iter().map(|p| &*p.ty).collect(),
    });
    let name_str = name.to_string();
    Ok(quote! {
        #(#other_attrs)*
        #[doc(hidden)]
        #sig #block

        #factory

        ::air_tight::__private::inventory::submit! {
            ::air_tight::registry::HandlerEntry {
                module_path: ::core::module_path!(),
                name: #name_str,
                kind: ::air_tight::registry::HandlerKind::Public,
                condition: "",
                keys: ::std::vec::Vec::new,
                resource_kinds: ::std::vec::Vec::new,
            }
        }
    })
}
