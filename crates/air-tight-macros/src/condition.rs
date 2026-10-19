//! The condition grammar of `#[permission(...)]` (spec section 1.7).
//!
//! ```text
//! expr    := and ("||" and)*
//! and     := unary ("&&" unary)*
//! unary   := "!" unary | primary
//! primary := "(" expr ")" | ("all" | "any") "(" expr ("," expr)* ","? ")"
//!          | Category "(" action ")"
//! action  := same shape over action variant idents, without all/any
//! ```

use proc_macro2::TokenStream;
use quote::{quote, quote_spanned};
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::{Ident, Token, parenthesized};

/// A parsed condition tree.
pub(crate) enum Cond {
    Leaf { category: Ident, action: Ident },
    All(Vec<Cond>),
    Any(Vec<Cond>),
    Not(Box<Cond>),
}

/// A parsed expression over action variants inside one category.
enum ActionExpr {
    Action(Ident),
    All(Vec<ActionExpr>),
    Any(Vec<ActionExpr>),
    Not(Box<ActionExpr>),
}

impl Parse for Cond {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        parse_or(input, &parse_primary)
    }
}

impl Parse for ActionExpr {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        parse_or(input, &parse_action_primary)
    }
}

/// Shared trait so the binary-operator layers work for both trees.
trait Tree: Sized {
    fn all(v: Vec<Self>) -> Self;
    fn any(v: Vec<Self>) -> Self;
    fn not(v: Self) -> Self;
}

impl Tree for Cond {
    fn all(v: Vec<Self>) -> Self {
        Cond::All(v)
    }
    fn any(v: Vec<Self>) -> Self {
        Cond::Any(v)
    }
    fn not(v: Self) -> Self {
        Cond::Not(Box::new(v))
    }
}

impl Tree for ActionExpr {
    fn all(v: Vec<Self>) -> Self {
        ActionExpr::All(v)
    }
    fn any(v: Vec<Self>) -> Self {
        ActionExpr::Any(v)
    }
    fn not(v: Self) -> Self {
        ActionExpr::Not(Box::new(v))
    }
}

type Primary<T> = dyn Fn(ParseStream<'_>) -> syn::Result<T>;

fn parse_or<T: Tree>(input: ParseStream<'_>, primary: &Primary<T>) -> syn::Result<T> {
    let mut items = vec![parse_and(input, primary)?];
    while input.peek(Token![||]) {
        input.parse::<Token![||]>()?;
        items.push(parse_and(input, primary)?);
    }
    Ok(if items.len() == 1 {
        items.pop().unwrap()
    } else {
        T::any(items)
    })
}

fn parse_and<T: Tree>(input: ParseStream<'_>, primary: &Primary<T>) -> syn::Result<T> {
    let mut items = vec![parse_unary(input, primary)?];
    while input.peek(Token![&&]) {
        input.parse::<Token![&&]>()?;
        items.push(parse_unary(input, primary)?);
    }
    Ok(if items.len() == 1 {
        items.pop().unwrap()
    } else {
        T::all(items)
    })
}

fn parse_unary<T: Tree>(input: ParseStream<'_>, primary: &Primary<T>) -> syn::Result<T> {
    if input.peek(Token![!]) {
        input.parse::<Token![!]>()?;
        return Ok(T::not(parse_unary(input, primary)?));
    }
    primary(input)
}

fn parse_primary(input: ParseStream<'_>) -> syn::Result<Cond> {
    if input.peek(syn::token::Paren) {
        let inner;
        parenthesized!(inner in input);
        let c: Cond = inner.parse()?;
        expect_empty(&inner)?;
        return Ok(c);
    }
    let head: Ident = input.parse().map_err(|e| {
        syn::Error::new(
            e.span(),
            "expected a permission like `Workspace(Read)`, `all(...)`, `any(...)`, `!` or `(`",
        )
    })?;
    if !input.peek(syn::token::Paren) {
        return Err(syn::Error::new(
            head.span(),
            format!("expected `(` after `{head}`; write `{head}(Action)`"),
        ));
    }
    let inner;
    parenthesized!(inner in input);
    match head.to_string().as_str() {
        "all" | "any" => {
            let items = Punctuated::<Cond, Token![,]>::parse_terminated(&inner)?;
            let items: Vec<Cond> = items.into_iter().collect();
            if items.is_empty() {
                return Err(syn::Error::new(
                    head.span(),
                    format!("`{head}()` needs at least one condition"),
                ));
            }
            Ok(if head == "all" {
                Cond::All(items)
            } else {
                Cond::Any(items)
            })
        }
        _ => {
            let expr: ActionExpr = inner.parse()?;
            expect_empty(&inner)?;
            Ok(expr.into_cond(&head))
        }
    }
}

fn parse_action_primary(input: ParseStream<'_>) -> syn::Result<ActionExpr> {
    if input.peek(syn::token::Paren) {
        let inner;
        parenthesized!(inner in input);
        let e: ActionExpr = inner.parse()?;
        expect_empty(&inner)?;
        return Ok(e);
    }
    let action: Ident = input
        .parse()
        .map_err(|e| syn::Error::new(e.span(), "expected an action variant like `Read`"))?;
    if input.peek(syn::token::Paren) {
        return Err(syn::Error::new(
            action.span(),
            "nested categories are not allowed inside a category; use `all(...)`/`any(...)` or `&&`/`||` between categories",
        ));
    }
    Ok(ActionExpr::Action(action))
}

fn expect_empty(input: ParseStream<'_>) -> syn::Result<()> {
    if input.is_empty() {
        Ok(())
    } else {
        Err(input.error(
            "unexpected tokens; combine conditions with `&&`, `||`, `!`, `all(...)` or `any(...)`",
        ))
    }
}

impl ActionExpr {
    fn into_cond(self, category: &Ident) -> Cond {
        match self {
            ActionExpr::Action(action) => Cond::Leaf {
                category: category.clone(),
                action,
            },
            ActionExpr::All(v) => Cond::All(v.into_iter().map(|e| e.into_cond(category)).collect()),
            ActionExpr::Any(v) => Cond::Any(v.into_iter().map(|e| e.into_cond(category)).collect()),
            ActionExpr::Not(e) => Cond::Not(Box::new(e.into_cond(category))),
        }
    }
}

impl Cond {
    /// Emits a const expression of type `::air_tight::Condition<#perms>`.
    pub(crate) fn to_tokens(&self, perms: &TokenStream) -> TokenStream {
        match self {
            Cond::Leaf { category, action } => {
                let (c, a) = (category.to_string(), action.to_string());
                let msg = format!(
                    "air_tight: `{c}({a})` is not a permission: no category variant `{c}` wrapping an action variant `{a}`"
                );
                let span = category
                    .span()
                    .join(action.span())
                    .unwrap_or_else(|| category.span());
                quote_spanned! {span=>
                    ::air_tight::Condition::Leaf(
                        match <#perms>::__air_tight_lookup(#c, #a) {
                            ::core::option::Option::Some(p) => p,
                            ::core::option::Option::None => ::core::panic!(#msg),
                        }
                    )
                }
            }
            Cond::All(v) => {
                let items = v.iter().map(|c| c.to_tokens(perms));
                quote!(::air_tight::Condition::All(&[#(#items),*]))
            }
            Cond::Any(v) => {
                let items = v.iter().map(|c| c.to_tokens(perms));
                quote!(::air_tight::Condition::Any(&[#(#items),*]))
            }
            Cond::Not(c) => {
                let inner = c.to_tokens(perms);
                quote!(::air_tight::Condition::Not(&#inner))
            }
        }
    }

    /// `(category, action)` names of all leaves, for docs and diagnostics.
    pub(crate) fn describe(&self) -> String {
        match self {
            Cond::Leaf { category, action } => format!("{category}({action})"),
            Cond::All(v) => format!(
                "all({})",
                v.iter().map(Cond::describe).collect::<Vec<_>>().join(", ")
            ),
            Cond::Any(v) => format!(
                "any({})",
                v.iter().map(Cond::describe).collect::<Vec<_>>().join(", ")
            ),
            Cond::Not(c) => format!("!{}", c.describe()),
        }
    }
}
