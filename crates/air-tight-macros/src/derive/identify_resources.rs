//! `#[derive(IdentifyResources)]`: collects resource scopes from explicitly
//! marked fields of a struct or enum.

use proc_macro2::TokenStream;
use quote::{format_ident, quote, quote_spanned};
use syn::spanned::Spanned;
use syn::{Data, DeriveInput, Fields, Ident, Member, Path, Type, WherePredicate};

use crate::attrs::{self, Allowed};

const ALLOWED: Allowed = Allowed {
    key: false,
    resource: true,
    nested: true,
};

enum Role {
    Resource(Path),
    Nested,
}

struct Marked<'a> {
    member: Member,
    binding: Ident,
    ty: &'a Type,
    role: Role,
}

/// Expands the derive.
pub(crate) fn expand(input: &DeriveInput) -> syn::Result<TokenStream> {
    attrs::forbid(&input.attrs, "the type itself; put them on fields")?;
    let mut predicates: Vec<WherePredicate> = Vec::new();

    let body = match &input.data {
        Data::Struct(s) => {
            let marked = marked_fields(&s.fields)?;
            add_predicates(&marked, &mut predicates);
            let pattern = pattern(quote!(Self), &s.fields, &marked);
            let pushes = marked.iter().map(push);
            quote! {
                #[allow(unused_variables)]
                let #pattern = self;
                #(#pushes)*
            }
        }
        Data::Enum(e) => {
            let mut arms = Vec::new();
            for v in &e.variants {
                attrs::forbid(&v.attrs, "enum variants; put them on the variant's fields")?;
                let marked = marked_fields(&v.fields)?;
                add_predicates(&marked, &mut predicates);
                let ident = &v.ident;
                let pattern = pattern(quote!(Self::#ident), &v.fields, &marked);
                let pushes = marked.iter().map(push);
                arms.push(quote! { #pattern => { #(#pushes)* } });
            }
            quote! {
                #[allow(unreachable_patterns, unused_variables)]
                match self {
                    #(#arms)*
                    _ => {}
                }
            }
        }
        Data::Union(u) => {
            return Err(syn::Error::new(
                u.union_token.span,
                "#[derive(IdentifyResources)] does not support unions",
            ));
        }
    };

    let name = &input.ident;
    let (impl_g, ty_g, where_clause) = input.generics.split_for_impl();
    let mut where_clause = where_clause
        .cloned()
        .unwrap_or_else(|| syn::parse_quote!(where));
    if !input.generics.params.is_empty() {
        where_clause.predicates.extend(predicates);
    }
    Ok(quote! {
        impl #impl_g ::air_tight::IdentifyResources for #name #ty_g #where_clause {
            fn resource_scopes(&self) -> ::std::vec::Vec<::air_tight::ResourceScope> {
                let mut __out = ::std::vec::Vec::new();
                #body
                __out
            }
        }
    })
}

fn marked_fields(fields: &Fields) -> syn::Result<Vec<Marked<'_>>> {
    let mut out = Vec::new();
    for (i, f) in fields.iter().enumerate() {
        let a = attrs::parse(&f.attrs, ALLOWED)?;
        let role = match (a.resource, a.nested) {
            (Some(_), Some(span)) => {
                return Err(syn::Error::new(
                    span,
                    "a field cannot be both `resource = ...` and `nested`",
                ));
            }
            (Some(path), None) => Role::Resource(path),
            (None, Some(_)) => Role::Nested,
            (None, None) => continue,
        };
        let member = match &f.ident {
            Some(id) => Member::Named(id.clone()),
            None => Member::Unnamed(i.into()),
        };
        out.push(Marked {
            member,
            binding: format_ident!("__f{}", i),
            ty: &f.ty,
            role,
        });
    }
    Ok(out)
}

fn pattern(path: TokenStream, fields: &Fields, marked: &[Marked<'_>]) -> TokenStream {
    let members = marked.iter().map(|m| &m.member);
    let bindings = marked.iter().map(|m| &m.binding);
    match fields {
        Fields::Unit => quote!(#path),
        _ => quote!(#path { #(#members: #bindings,)* .. }),
    }
}

fn push(m: &Marked<'_>) -> TokenStream {
    let binding = &m.binding;
    let span = m.ty.span();
    match &m.role {
        Role::Resource(kind) => {
            let ty = m.ty;
            let ids = quote_spanned!(span=> <#ty as ::air_tight::ResourceIds>::ids(#binding));
            let key =
                quote_spanned!(kind.span()=> <#kind as ::air_tight::ResourceKindMarker>::key());
            quote! {
                let __kind = #key;
                for __id in #ids {
                    __out.push(::air_tight::ResourceScope::new(__kind.clone(), __id));
                }
            }
        }
        Role::Nested => {
            let ty = m.ty;
            let scopes = quote_spanned!(span=>
                <#ty as ::air_tight::IdentifyResources>::resource_scopes(#binding)
            );
            quote! { __out.extend(#scopes); }
        }
    }
}

fn add_predicates(marked: &[Marked<'_>], out: &mut Vec<WherePredicate>) {
    for m in marked {
        let ty = m.ty;
        out.push(match m.role {
            Role::Resource(_) => syn::parse_quote!(#ty: ::air_tight::ResourceIds),
            Role::Nested => syn::parse_quote!(#ty: ::air_tight::IdentifyResources),
        });
    }
}
