//! Handler mode of `#[permission(...)]`.

use proc_macro2::TokenStream;
use quote::{format_ident, quote, quote_spanned};
use syn::{FnArg, Ident, ItemFn, Pat, PatType, ReturnType, parse_quote};

use super::args::{ErrorMode, PermissionArgs};
use super::factory::{self, Factory, Wrapper};
use super::{params, perms_path, returns, user_path};

/// Expands a protected axum handler.
pub(crate) fn expand(args: PermissionArgs, func: ItemFn) -> syn::Result<TokenStream> {
    let ItemFn {
        attrs,
        vis,
        sig,
        block,
    } = func;
    if sig.asyncness.is_none() {
        return Err(syn::Error::new(
            sig.fn_token.span,
            "#[air_tight::permission] handlers must be `async fn`; use `kind = fn` for other functions",
        ));
    }
    if !sig.generics.params.is_empty() {
        return Err(syn::Error::new_spanned(
            &sig.generics,
            "#[air_tight::permission] handlers cannot be generic",
        ));
    }
    if let Some(v) = &sig.variadic {
        return Err(syn::Error::new_spanned(
            v,
            "variadic handlers are not supported",
        ));
    }

    let mode = match args.error {
        Some((mode, _)) => mode,
        None if returns::is_result(&sig.output) => ErrorMode::Result,
        None => ErrorMode::Response,
    };

    let axum = quote!(::air_tight::__private::axum);
    let perms = perms_path();
    let user_ty = user_path();
    let mut user_params = params::typed(sig.inputs.clone())?;

    // Auto-include AirTightPath / AirTightQuery params; rename complex patterns.
    let mut scope_exprs: Vec<Ident> = Vec::new();
    let mut deferred = Vec::new();
    for (i, p) in user_params.iter_mut().enumerate() {
        if !params::is_auto_resource(&p.ty) {
            continue;
        }
        match &*p.pat {
            Pat::Ident(pi) if pi.subpat.is_none() => scope_exprs.push(pi.ident.clone()),
            _ => {
                let fresh = params::auto_ident(i);
                let pat = std::mem::replace(&mut *p.pat, parse_quote!(#fresh));
                deferred.push(quote!(let #pat = #fresh;));
                scope_exprs.push(fresh);
            }
        }
    }
    for name in &args.resources {
        let b = params::find(&user_params, name, "a resource")?.clone();
        if !scope_exprs.contains(&b) {
            scope_exprs.push(b);
        }
    }

    let client = match &args.state {
        Some(name) => params::find(&user_params, name, "the air_tight state")?.clone(),
        None => format_ident!("__air_tight"),
    };
    let user = match &args.user {
        Some(name) => params::find(&user_params, name, "the user")?.clone(),
        None => format_ident!("__air_tight_user"),
    };

    let mut all_params: Vec<PatType> = Vec::new();
    if args.state.is_none() {
        all_params.push(parse_quote! {
            #axum::extract::State(__air_tight): #axum::extract::State<::air_tight::AirTight<#perms, #user_ty>>
        });
    }
    if args.user.is_none() {
        all_params.push(parse_quote! {
            ::air_tight::CurrentUser(__air_tight_user): ::air_tight::CurrentUser<#user_ty>
        });
    }
    all_params.extend(user_params);

    let name = &sig.ident;
    let inner = format_ident!("__air_tight_handler_{}", name);
    let cond_static = format_ident!("__AIR_TIGHT_CONDITION_{}", name.to_string().to_uppercase());
    let cond_tokens = args.cond.to_tokens(&perms);
    let cond_text = args.cond.describe();

    let scopes = quote! {{
        let mut __s = ::std::vec::Vec::new();
        #( __s.extend(::air_tight::IdentifyResources::resource_scopes(&#scope_exprs)); )*
        __s
    }};
    let call = quote_spanned! {sig.ident.span()=>
        #client.authorize_async(&#user, &#cond_static, #scopes).await
    };

    let stmts = &block.stmts;
    let (output, body) = match mode {
        ErrorMode::Result => (
            sig.output.clone(),
            quote! {
                let __air_tight_granted = #call?;
                #(#deferred)*
                #[allow(unused_variables)]
                let granted = __air_tight_granted;
                #(#stmts)*
            },
        ),
        ErrorMode::Response => {
            // Concrete return types annotate the body's value. `impl Trait`
            // return types cannot, so they go into a never-called helper that
            // keeps the user's tokens (and their imports) in use.
            let (annot, keep) = match &sig.output {
                ReturnType::Type(_, ty) if !returns::mentions_impl(ty) => (quote!(: #ty), quote!()),
                ReturnType::Type(_, ty) => (
                    quote!(),
                    quote! {
                        #[allow(dead_code)]
                        fn __air_tight_declared_return() -> #ty {
                            ::air_tight::__private::unreachable_value::<::core::convert::Infallible>()
                        }
                    },
                ),
                ReturnType::Default => (quote!(), quote!()),
            };
            (
                parse_quote!(-> #axum::response::Response),
                quote! {
                    #keep
                    let __air_tight_granted = match #call {
                        ::core::result::Result::Ok(g) => g,
                        ::core::result::Result::Err(e) => {
                            return #axum::response::IntoResponse::into_response(e);
                        }
                    };
                    #(#deferred)*
                    #[allow(unused_variables)]
                    let granted = __air_tight_granted;
                    let __air_tight_out #annot = async move { #(#stmts)* }.await;
                    #axum::response::IntoResponse::into_response(__air_tight_out)
                },
            )
        }
    };

    let (doc_attrs, other_attrs) = factory::split_docs(attrs);
    let inputs = all_params.iter().cloned().map(FnArg::Typed);
    let factory = factory::emit(&Factory {
        wrapper: Wrapper::Guarded,
        vis: &vis,
        name,
        inner: &inner,
        doc_attrs: &doc_attrs,
        access_doc: format!("Requires `{cond_text}`."),
        param_tys: all_params.iter().map(|p| &*p.ty).collect(),
    });
    let marker = format_ident!("__Protected_{}", name);
    let name_str = name.to_string();

    Ok(quote! {
        #[allow(non_upper_case_globals)]
        static #cond_static: ::air_tight::Condition<#perms> = #cond_tokens;

        #(#other_attrs)*
        #[doc(hidden)]
        async fn #inner(#(#inputs),*) #output {
            #body
        }

        #factory

        #[doc(hidden)]
        #[allow(non_camel_case_types)]
        #vis struct #marker;

        impl ::air_tight::router::Protected for #marker {
            const HANDLER: &'static str = ::core::concat!(::core::module_path!(), "::", #name_str);
            const CONDITION: &'static str = #cond_text;
        }

        ::air_tight::__private::inventory::submit! {
            ::air_tight::registry::HandlerEntry {
                module_path: ::core::module_path!(),
                name: #name_str,
                kind: ::air_tight::registry::HandlerKind::Protected,
                condition: #cond_text,
                keys: || {
                    #cond_static
                        .leaves()
                        .into_iter()
                        .map(|p| ::air_tight::PermissionCategory::key(p).to_string())
                        .collect()
                },
                resource_kinds: || {
                    let mut kinds: ::std::vec::Vec<::std::string::String> = #cond_static
                        .leaves()
                        .into_iter()
                        .filter_map(|p| ::air_tight::PermissionCategory::resource_kind(p))
                        .map(|k| k.to_string())
                        .collect();
                    kinds.sort();
                    kinds.dedup();
                    kinds
                },
            }
        }
    })
}
