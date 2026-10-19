//! Parsing of `#[air_tight(...)]` helper attributes on variants and fields.

use syn::spanned::Spanned;
use syn::{Attribute, LitStr, Path};

/// Which helper arguments are accepted in a given position.
#[derive(Clone, Copy)]
pub(crate) struct Allowed {
    pub key: bool,
    pub resource: bool,
    pub nested: bool,
}

impl Allowed {
    fn names(self) -> String {
        let mut v = Vec::new();
        if self.key {
            v.push("`key = \"...\"`");
        }
        if self.resource {
            v.push("`resource = Kind`");
        }
        if self.nested {
            v.push("`nested`");
        }
        v.join(", ")
    }
}

/// Parsed helper arguments.
#[derive(Default)]
pub(crate) struct HelperAttrs {
    pub key: Option<LitStr>,
    pub resource: Option<Path>,
    pub nested: Option<proc_macro2::Span>,
}

/// Parses every `#[air_tight(...)]` attribute in `attrs`, rejecting arguments
/// not listed in `allowed` and duplicates.
pub(crate) fn parse(attrs: &[Attribute], allowed: Allowed) -> syn::Result<HelperAttrs> {
    let mut out = HelperAttrs::default();
    for attr in attrs.iter().filter(|a| a.path().is_ident("air_tight")) {
        attr.parse_nested_meta(|meta| {
            let span = meta.path.span();
            let dup = || syn::Error::new(span, "duplicate air_tight argument");
            if allowed.key && meta.path.is_ident("key") {
                if out.key.is_some() {
                    return Err(dup());
                }
                let lit: LitStr = meta.value()?.parse()?;
                validate_key_segment(&lit)?;
                out.key = Some(lit);
            } else if allowed.resource && meta.path.is_ident("resource") {
                if out.resource.is_some() {
                    return Err(dup());
                }
                out.resource = Some(meta.value()?.parse()?);
            } else if allowed.nested && meta.path.is_ident("nested") {
                if out.nested.is_some() {
                    return Err(dup());
                }
                out.nested = Some(span);
            } else {
                return Err(syn::Error::new(
                    span,
                    format!(
                        "unknown air_tight argument here; expected one of: {}",
                        allowed.names()
                    ),
                ));
            }
            Ok(())
        })?;
    }
    Ok(out)
}

/// Rejects attributes of the form `#[air_tight(...)]` entirely.
pub(crate) fn forbid(attrs: &[Attribute], what: &str) -> syn::Result<()> {
    match attrs.iter().find(|a| a.path().is_ident("air_tight")) {
        Some(a) => Err(syn::Error::new_spanned(
            a,
            format!("air_tight attributes are not supported on {what}"),
        )),
        None => Ok(()),
    }
}

fn validate_key_segment(lit: &LitStr) -> syn::Result<()> {
    let v = lit.value();
    if v.is_empty() || v.contains("::") || v.contains('*') || v.chars().any(char::is_whitespace) {
        return Err(syn::Error::new(
            lit.span(),
            "key override must be a non-empty segment without `::`, `*` or whitespace",
        ));
    }
    Ok(())
}
