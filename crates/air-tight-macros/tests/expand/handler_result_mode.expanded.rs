#[allow(non_upper_case_globals)]
static __AIR_TIGHT_CONDITION_CREATE_INVOICE: ::air_tight::Condition<
    crate::__air_tight::Perms,
> = ::air_tight::Condition::All(
    &[
        ::air_tight::Condition::Leaf(
            match <crate::__air_tight::Perms>::__air_tight_lookup(
                "Workspace",
                "Update",
            ) {
                ::core::option::Option::Some(p) => p,
                ::core::option::Option::None => {
                    ::core::panic!(
                        "air_tight: `Workspace(Update)` is not a permission: no category variant `Workspace` wrapping an action variant `Update`"
                    )
                }
            },
        ),
        ::air_tight::Condition::Leaf(
            match <crate::__air_tight::Perms>::__air_tight_lookup("Invoice", "Create") {
                ::core::option::Option::Some(p) => p,
                ::core::option::Option::None => {
                    ::core::panic!(
                        "air_tight: `Invoice(Create)` is not a permission: no category variant `Invoice` wrapping an action variant `Create`"
                    )
                }
            },
        ),
    ],
);
#[doc(hidden)]
async fn __air_tight_handler_create_invoice(
    ::air_tight::__private::axum::extract::State(
        __air_tight,
    ): ::air_tight::__private::axum::extract::State<
        ::air_tight::AirTight<crate::__air_tight::Perms, crate::__air_tight::User>,
    >,
    ::air_tight::CurrentUser(
        __air_tight_user,
    ): ::air_tight::CurrentUser<crate::__air_tight::User>,
    Json(body): Json<CreateInvoice>,
) -> Result<Json<Invoice>, AppError> {
    let __air_tight_granted = __air_tight
        .authorize_async(
            &__air_tight_user,
            &__AIR_TIGHT_CONDITION_CREATE_INVOICE,
            {
                let mut __s = ::std::vec::Vec::new();
                __s.extend(::air_tight::IdentifyResources::resource_scopes(&body));
                __s
            },
        )
        .await?;
    #[allow(unused_variables)]
    let granted = __air_tight_granted;
    Ok(Json(save(body)?))
}
/// Creates an invoice.
///
///Requires `all(Workspace(Update), Invoice(Create))`.
///
///Route with `air_tight::routing::get(create_invoice)` (or `post`, `patch`, ...). The handler body lives in a hidden function; this one only hands it to the typed router.
pub fn create_invoice<__S, __M>() -> ::air_tight::Guarded<
    impl ::air_tight::__private::axum::handler::Handler<
        (
            __M,
            ::air_tight::__private::axum::extract::State<
                ::air_tight::AirTight<
                    crate::__air_tight::Perms,
                    crate::__air_tight::User,
                >,
            >,
            ::air_tight::CurrentUser<crate::__air_tight::User>,
            Json<CreateInvoice>,
        ),
        __S,
    >,
>
where
    __S: ::core::clone::Clone + ::core::marker::Send + ::core::marker::Sync + 'static,
    ::air_tight::__private::axum::extract::State<
        ::air_tight::AirTight<crate::__air_tight::Perms, crate::__air_tight::User>,
    >: ::air_tight::__private::axum::extract::FromRequestParts<__S>,
    ::air_tight::CurrentUser<
        crate::__air_tight::User,
    >: ::air_tight::__private::axum::extract::FromRequestParts<__S>,
    Json<CreateInvoice>: ::air_tight::__private::axum::extract::FromRequest<__S, __M>,
{
    ::air_tight::Guarded::__new(__air_tight_handler_create_invoice)
}
#[doc(hidden)]
#[allow(non_camel_case_types)]
pub struct __Protected_create_invoice;
impl ::air_tight::router::Protected for __Protected_create_invoice {
    const HANDLER: &'static str = ::core::concat!(
        ::core::module_path!(), "::", "create_invoice"
    );
    const CONDITION: &'static str = "all(Workspace(Update), Invoice(Create))";
}
::air_tight::__private::inventory::submit! {
    ::air_tight::registry::HandlerEntry { module_path : ::core::module_path!(), name :
    "create_invoice", kind : ::air_tight::registry::HandlerKind::Protected, condition :
    "all(Workspace(Update), Invoice(Create))", keys : || {
    __AIR_TIGHT_CONDITION_CREATE_INVOICE.leaves().into_iter().map(| p |
    ::air_tight::PermissionCategory::key(p).to_string()).collect() }, resource_kinds : ||
    { let mut kinds : ::std::vec::Vec < ::std::string::String > =
    __AIR_TIGHT_CONDITION_CREATE_INVOICE.leaves().into_iter().filter_map(| p |
    ::air_tight::PermissionCategory::resource_kind(p)).map(| k | k.to_string())
    .collect(); kinds.sort(); kinds.dedup(); kinds }, }
}
