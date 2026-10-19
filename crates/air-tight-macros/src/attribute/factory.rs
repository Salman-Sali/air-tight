//! The routable factory function that replaces a handler's name.
//!
//! `#[permission]` and `#[unrestricted]` move the handler body into a hidden
//! inner function and reuse the handler's name for a factory returning
//! `Guarded<impl Handler<..>>` (or `Public<..>`). The typed routing functions
//! take that factory (`get(my_handler)` reads as in axum), and a plain handler
//! cannot be passed because it returns a future, not a `Guarded`.

use proc_macro2::TokenStream;
use quote::quote;
use syn::{Attribute, Ident, Type, Visibility};

/// Which wrapper the factory returns.
#[derive(Clone, Copy)]
pub(crate) enum Wrapper {
    Guarded,
    Public,
}

/// Inputs for [`emit`].
pub(crate) struct Factory<'a> {
    pub wrapper: Wrapper,
    pub vis: &'a Visibility,
    pub name: &'a Ident,
    pub inner: &'a Ident,
    pub doc_attrs: &'a [Attribute],
    /// One line describing access, appended after the user's docs.
    pub access_doc: String,
    pub param_tys: Vec<&'a Type>,
}

/// Emits the factory function.
pub(crate) fn emit(f: &Factory<'_>) -> TokenStream {
    let axum = quote!(::air_tight::__private::axum);
    let wrapper = match f.wrapper {
        Wrapper::Guarded => quote!(::air_tight::Guarded),
        Wrapper::Public => quote!(::air_tight::Public),
    };
    let (vis, name, inner, docs) = (f.vis, f.name, f.inner, f.doc_attrs);
    let s_bound =
        quote!(__S: ::core::clone::Clone + ::core::marker::Send + ::core::marker::Sync + 'static);
    let access_doc = &f.access_doc;
    let routing_doc = format!(
        "Route with `air_tight::routing::get({name})` (or `post`, `patch`, ...). The handler \
         body lives in a hidden function; this one only hands it to the typed router."
    );
    match f.param_tys.split_last() {
        None => quote! {
            #(#docs)*
            #[doc = ""]
            #[doc = #access_doc]
            #[doc = ""]
            #[doc = #routing_doc]
            #vis fn #name<__S>() -> #wrapper<impl #axum::handler::Handler<((),), __S>>
            where
                #s_bound,
            {
                #wrapper::__new(#inner)
            }
        },
        Some((last, init)) => {
            let all = &f.param_tys;
            quote! {
                #(#docs)*
                #[doc = ""]
                #[doc = #access_doc]
                #[doc = ""]
                #[doc = #routing_doc]
                #vis fn #name<__S, __M>() -> #wrapper<impl #axum::handler::Handler<(__M, #(#all,)*), __S>>
                where
                    #s_bound,
                    #(#init: #axum::extract::FromRequestParts<__S>,)*
                    #last: #axum::extract::FromRequest<__S, __M>,
                {
                    #wrapper::__new(#inner)
                }
            }
        }
    }
}

/// Splits `#[doc]` attributes (for the factory) from the rest (for the inner fn).
pub(crate) fn split_docs(attrs: Vec<Attribute>) -> (Vec<Attribute>, Vec<Attribute>) {
    attrs.into_iter().partition(|a| a.path().is_ident("doc"))
}
