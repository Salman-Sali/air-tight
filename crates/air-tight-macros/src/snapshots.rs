//! Expansion snapshot tests: each case expands an attribute and compares the
//! pretty-printed output with `tests/expand/<case>.expanded.rs`. Set
//! `AIR_TIGHT_UPDATE_SNAPSHOTS=1` to rewrite the fixtures.

use std::path::PathBuf;

use proc_macro2::TokenStream;
use quote::quote;

use crate::attribute;

fn check(case: &str, expanded: syn::Result<TokenStream>) {
    let tokens = expanded.unwrap_or_else(|e| panic!("{case}: {e}"));
    let file: syn::File = syn::parse2(tokens).unwrap_or_else(|e| panic!("{case}: {e}"));
    let actual = prettyplease::unparse(&file);
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/expand")
        .join(format!("{case}.expanded.rs"));
    if std::env::var_os("AIR_TIGHT_UPDATE_SNAPSHOTS").is_some() {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, &actual).unwrap();
        return;
    }
    let expected = std::fs::read_to_string(&path).unwrap_or_else(|_| {
        panic!(
            "missing {}; run with AIR_TIGHT_UPDATE_SNAPSHOTS=1",
            path.display()
        )
    });
    assert_eq!(
        actual, expected,
        "snapshot {case} changed; rerun with AIR_TIGHT_UPDATE_SNAPSHOTS=1 if intended"
    );
}

#[test]
fn handler_result_mode() {
    check(
        "handler_result_mode",
        attribute::expand_permission(
            quote!(Workspace(Update) && Invoice(Create), resources = [body]),
            quote! {
                /// Creates an invoice.
                pub async fn create_invoice(Json(body): Json<CreateInvoice>) -> Result<Json<Invoice>, AppError> {
                    Ok(Json(save(body)?))
                }
            },
        ),
    );
}

#[test]
fn handler_response_mode_auto_path() {
    check(
        "handler_response_mode_auto_path",
        attribute::expand_permission(
            quote!(Workspace(Read || Update)),
            quote! {
                async fn get_workspace(AirTightPath(Path(id)): AirTightPath<Uuid, Workspace>) -> impl IntoResponse {
                    Json(id)
                }
            },
        ),
    );
}

#[test]
fn handler_overrides() {
    check(
        "handler_overrides",
        attribute::expand_permission(
            quote!(
                any(Billing(Read), !Workspace(Delete)),
                state = client,
                user = me,
                error = response
            ),
            quote! {
                async fn billing(State(client): State<Client>, Extension(me): Extension<User>) -> Response {
                    StatusCode::OK.into_response()
                }
            },
        ),
    );
}

#[test]
fn function_mode_async_method() {
    check(
        "function_mode_async_method",
        attribute::expand_permission(
            quote!(Workspace(Update), kind = fn, resources = [ws]),
            quote! {
                pub async fn rename(&self, ws: WorkspaceRef, name: String) -> Result<(), AppError> {
                    self.repo.rename(ws, name)
                }
            },
        ),
    );
}

#[test]
fn unrestricted_handler() {
    let func: syn::ItemFn = syn::parse2(quote! {
        pub async fn health() -> &'static str { "ok" }
    })
    .unwrap();
    check(
        "unrestricted_handler",
        attribute::expand_unrestricted(TokenStream::new(), func),
    );
}
