//! `#[air_tight::permission(...)]` and `#[air_tight::unrestricted]`.

mod args;
mod factory;
mod function;
mod handler;
mod params;
mod returns;
mod unrestricted;

pub(crate) use args::PermissionArgs;
pub(crate) use function::expand as expand_fn;
pub(crate) use handler::expand as expand_handler;
pub(crate) use unrestricted::expand as expand_unrestricted;

use proc_macro2::TokenStream;
use quote::quote;

/// `crate::__air_tight::Perms`, emitted by `air_tight::setup!`.
pub(crate) fn perms_path() -> TokenStream {
    quote!(crate::__air_tight::Perms)
}

/// `crate::__air_tight::User`, emitted by `air_tight::setup!`.
pub(crate) fn user_path() -> TokenStream {
    quote!(crate::__air_tight::User)
}

/// Expands `#[permission(...)]` in the mode chosen by its arguments.
pub(crate) fn expand_permission(args: TokenStream, item: TokenStream) -> syn::Result<TokenStream> {
    let args: PermissionArgs = syn::parse2(args)?;
    let func: syn::ItemFn = syn::parse2(item)?;
    if args.kind_fn.is_some() {
        expand_fn(args, func)
    } else {
        expand_handler(args, func)
    }
}
