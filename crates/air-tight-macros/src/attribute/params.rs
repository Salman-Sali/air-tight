//! Locating handler parameters by binding name.

use quote::format_ident;
use syn::{FnArg, Ident, Pat, PatType, Type};

/// The plain binding of a parameter pattern: `name` or `Wrapper(name)`.
pub(crate) fn binding(pat: &Pat) -> Option<&Ident> {
    match pat {
        Pat::Ident(p) if p.subpat.is_none() => Some(&p.ident),
        Pat::TupleStruct(ts) if ts.elems.len() == 1 => match &ts.elems[0] {
            Pat::Ident(p) if p.subpat.is_none() => Some(&p.ident),
            _ => None,
        },
        _ => None,
    }
}

/// Whether `name` is bound anywhere inside `pat`.
fn binds(pat: &Pat, name: &Ident) -> bool {
    match pat {
        Pat::Ident(p) => p.ident == *name || p.subpat.as_ref().is_some_and(|(_, s)| binds(s, name)),
        Pat::TupleStruct(t) => t.elems.iter().any(|p| binds(p, name)),
        Pat::Tuple(t) => t.elems.iter().any(|p| binds(p, name)),
        Pat::Struct(s) => s.fields.iter().any(|f| binds(&f.pat, name)),
        Pat::Slice(s) => s.elems.iter().any(|p| binds(p, name)),
        Pat::Reference(r) => binds(&r.pat, name),
        Pat::Paren(p) => binds(&p.pat, name),
        Pat::Type(t) => binds(&t.pat, name),
        Pat::Or(o) => o.cases.iter().any(|p| binds(p, name)),
        _ => false,
    }
}

/// Finds the parameter binding `name`, returning the plain ident to use.
pub(crate) fn find<'a>(params: &'a [PatType], name: &Ident, what: &str) -> syn::Result<&'a Ident> {
    for p in params {
        if let Some(b) = binding(&p.pat) {
            if b == name {
                return Ok(b);
            }
        } else if binds(&p.pat, name) {
            return Err(syn::Error::new(
                name.span(),
                format!(
                    "`{name}` must be bound to a plain name to be used as {what}: write `{name}: Type` \
                     or a one-level wrapper like `Json({name}): Json<Type>`"
                ),
            ));
        }
    }
    Err(syn::Error::new(
        name.span(),
        format!("no parameter binds `{name}` (listed as {what})"),
    ))
}

/// Whether the parameter type's last path segment is `AirTightPath` or `AirTightQuery`.
pub(crate) fn is_auto_resource(ty: &Type) -> bool {
    match ty {
        Type::Path(p) => p
            .path
            .segments
            .last()
            .is_some_and(|s| s.ident == "AirTightPath" || s.ident == "AirTightQuery"),
        _ => false,
    }
}

/// Typed parameters of a free function, rejecting receivers.
pub(crate) fn typed(inputs: impl IntoIterator<Item = FnArg>) -> syn::Result<Vec<PatType>> {
    inputs
        .into_iter()
        .map(|a| match a {
            FnArg::Typed(t) => Ok(t),
            FnArg::Receiver(r) => Err(syn::Error::new_spanned(
                r,
                "handlers cannot take `self`; use `kind = fn` for methods",
            )),
        })
        .collect()
}

/// Fresh ident for a renamed auto-detected parameter.
pub(crate) fn auto_ident(i: usize) -> Ident {
    format_ident!("__air_tight_auto_{}", i)
}
