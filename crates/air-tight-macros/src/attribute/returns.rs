//! Return type inspection.

use proc_macro2::{TokenStream, TokenTree};
use quote::ToTokens;
use syn::{ReturnType, Type};

/// Whether the return type is syntactically `Result<_, _>` (last segment).
pub(crate) fn is_result(output: &ReturnType) -> bool {
    match output {
        ReturnType::Type(_, ty) => match &**ty {
            Type::Path(p) => p.path.segments.last().is_some_and(|s| s.ident == "Result"),
            _ => false,
        },
        ReturnType::Default => false,
    }
}

/// Whether a type mentions `impl Trait` anywhere.
pub(crate) fn mentions_impl(ty: &Type) -> bool {
    fn walk(ts: TokenStream) -> bool {
        ts.into_iter().any(|tt| match tt {
            TokenTree::Ident(i) => i == "impl",
            TokenTree::Group(g) => walk(g.stream()),
            _ => false,
        })
    }
    walk(ty.to_token_stream())
}
