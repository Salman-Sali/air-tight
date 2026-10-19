//! Arguments of `#[permission(COND, resources = [..], kind = fn, error = ..,
//! state = .., user = ..)]`.

use proc_macro2::Span;
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::{Ident, Token, bracketed};

use crate::condition::Cond;

/// How an authorization failure leaves the function.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum ErrorMode {
    /// `return Err(From::from(e))` via `?`.
    Result,
    /// `return e.into_response()`.
    Response,
}

/// Parsed attribute arguments.
pub(crate) struct PermissionArgs {
    pub cond: Cond,
    pub resources: Vec<Ident>,
    pub kind_fn: Option<Span>,
    pub error: Option<(ErrorMode, Span)>,
    pub state: Option<Ident>,
    pub user: Option<Ident>,
}

impl Parse for PermissionArgs {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        if input.is_empty() {
            return Err(
                input.error("expected a condition, e.g. #[air_tight::permission(Workspace(Read))]")
            );
        }
        let cond: Cond = input.parse()?;
        let mut args = PermissionArgs {
            cond,
            resources: Vec::new(),
            kind_fn: None,
            error: None,
            state: None,
            user: None,
        };
        let mut seen: Vec<String> = Vec::new();
        while !input.is_empty() {
            input.parse::<Token![,]>().map_err(|e| {
                syn::Error::new(
                    e.span(),
                    "expected `,` then `resources = [..]`, `kind = fn`, `error = result|response`, \
                     `state = name` or `user = name`; combine conditions with `&&`, `||`, `all(..)`, `any(..)`",
                )
            })?;
            if input.is_empty() {
                break;
            }
            let key: Ident = if input.peek(Token![fn]) {
                return Err(input.error("expected an argument name"));
            } else {
                input.parse()?
            };
            let name = key.to_string();
            if seen.contains(&name) {
                return Err(syn::Error::new(
                    key.span(),
                    format!("duplicate argument `{name}`"),
                ));
            }
            seen.push(name.clone());
            input.parse::<Token![=]>()?;
            match name.as_str() {
                "resources" => {
                    let content;
                    bracketed!(content in input);
                    args.resources = Punctuated::<Ident, Token![,]>::parse_terminated(&content)?
                        .into_iter()
                        .collect();
                }
                "kind" => {
                    let tok: Token![fn] = input.parse().map_err(|e| {
                        syn::Error::new(e.span(), "the only supported kind is `kind = fn`")
                    })?;
                    args.kind_fn = Some(tok.span);
                }
                "error" => {
                    let v: Ident = input.parse()?;
                    let mode = match v.to_string().as_str() {
                        "result" => ErrorMode::Result,
                        "response" => ErrorMode::Response,
                        _ => {
                            return Err(syn::Error::new(
                                v.span(),
                                "expected `error = result` or `error = response`",
                            ));
                        }
                    };
                    args.error = Some((mode, v.span()));
                }
                "state" => args.state = Some(input.parse()?),
                "user" => args.user = Some(input.parse()?),
                _ => {
                    return Err(syn::Error::new(
                        key.span(),
                        format!(
                            "unknown argument `{name}`; expected `resources`, `kind`, `error`, `state` or `user`"
                        ),
                    ));
                }
            }
        }
        Ok(args)
    }
}
