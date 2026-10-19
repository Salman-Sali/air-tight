//! Function mode of `#[permission(..., kind = fn)]`: protect any function,
//! for example a service called from a handler.

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{FnArg, ItemFn, parse_quote};

use super::args::{ErrorMode, PermissionArgs};
use super::{params, perms_path, returns, user_path};

/// Expands a protected function.
pub(crate) fn expand(args: PermissionArgs, func: ItemFn) -> syn::Result<TokenStream> {
    if let Some((ErrorMode::Response, span)) = args.error {
        return Err(syn::Error::new(
            span,
            "`error = response` is not supported with `kind = fn`; return a `Result` whose error implements `From<air_tight::Error>`",
        ));
    }
    if let Some(s) = args.state.as_ref().or(args.user.as_ref()) {
        return Err(syn::Error::new(
            s.span(),
            "`state =` and `user =` are handler-mode options; `kind = fn` always injects its own parameters",
        ));
    }
    if args.error.is_none() && !returns::is_result(&func.sig.output) {
        return Err(syn::Error::new_spanned(
            &func.sig,
            "`kind = fn` requires a `Result<_, E>` return type where `E: From<air_tight::Error>` (or `error = result`)",
        ));
    }

    let ItemFn {
        mut attrs,
        vis,
        mut sig,
        block,
    } = func;
    let perms = perms_path();
    let user_ty = user_path();

    let typed: Vec<_> = sig
        .inputs
        .iter()
        .filter_map(|a| match a {
            FnArg::Typed(t) => Some(t.clone()),
            FnArg::Receiver(_) => None,
        })
        .collect();
    let mut scope_exprs = Vec::new();
    for name in &args.resources {
        scope_exprs.push(params::find(&typed, name, "a resource")?.clone());
    }

    let injected: [FnArg; 3] = [
        parse_quote!(__air_tight: &::air_tight::AirTight<#perms, #user_ty>),
        parse_quote!(__air_tight_user: &#user_ty),
        parse_quote!(__air_tight_scopes: ::std::vec::Vec<::air_tight::ResourceScope>),
    ];
    let at = usize::from(matches!(sig.inputs.first(), Some(FnArg::Receiver(_))));
    let mut inputs: Vec<FnArg> = sig.inputs.iter().cloned().collect();
    for (i, arg) in injected.into_iter().enumerate() {
        inputs.insert(at + i, arg);
    }
    sig.inputs = inputs.into_iter().collect();

    let cond_static = format_ident!("__AIR_TIGHT_CONDITION");
    let cond_tokens = args.cond.to_tokens(&perms);
    let cond_text = args.cond.describe();
    let scopes = quote! {{
        let mut __s = __air_tight_scopes;
        #( __s.extend(::air_tight::IdentifyResources::resource_scopes(&#scope_exprs)); )*
        __s
    }};
    let call = if sig.asyncness.is_some() {
        quote!(__air_tight.authorize_async(__air_tight_user, &#cond_static, #scopes).await)
    } else {
        quote!(__air_tight.authorize(__air_tight_user, &#cond_static, #scopes))
    };
    let doc = format!(
        "Requires `{cond_text}`. Parameters injected by `#[air_tight::permission(kind = fn)]`, \
         in front of the declared ones: `__air_tight: &AirTight<Perms, User>`, \
         `__air_tight_user: &User`, `__air_tight_scopes: Vec<ResourceScope>`."
    );
    attrs.push(parse_quote!(#[doc = ""]));
    attrs.push(parse_quote!(#[doc = #doc]));
    let stmts = &block.stmts;

    Ok(quote! {
        #(#attrs)*
        #vis #sig {
            #[allow(non_upper_case_globals)]
            static #cond_static: ::air_tight::Condition<#perms> = #cond_tokens;
            let __air_tight_granted = #call?;
            #[allow(unused_variables)]
            let granted = __air_tight_granted;
            #(#stmts)*
        }
    })
}
