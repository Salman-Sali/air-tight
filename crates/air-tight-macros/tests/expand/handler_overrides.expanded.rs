#[allow(non_upper_case_globals)]
static __AIR_TIGHT_CONDITION_BILLING: ::air_tight::Condition<
    crate::__air_tight::Perms,
> = ::air_tight::Condition::Any(
    &[
        ::air_tight::Condition::Leaf(
            match <crate::__air_tight::Perms>::__air_tight_lookup("Billing", "Read") {
                ::core::option::Option::Some(p) => p,
                ::core::option::Option::None => {
                    ::core::panic!(
                        "air_tight: `Billing(Read)` is not a permission: no category variant `Billing` wrapping an action variant `Read`"
                    )
                }
            },
        ),
        ::air_tight::Condition::Not(
            &::air_tight::Condition::Leaf(
                match <crate::__air_tight::Perms>::__air_tight_lookup(
                    "Workspace",
                    "Delete",
                ) {
                    ::core::option::Option::Some(p) => p,
                    ::core::option::Option::None => {
                        ::core::panic!(
                            "air_tight: `Workspace(Delete)` is not a permission: no category variant `Workspace` wrapping an action variant `Delete`"
                        )
                    }
                },
            ),
        ),
    ],
);
#[doc(hidden)]
async fn __air_tight_handler_billing(
    State(client): State<Client>,
    Extension(me): Extension<User>,
) -> ::air_tight::__private::axum::response::Response {
    let __air_tight_granted = match client
        .authorize_async(
            &me,
            &__AIR_TIGHT_CONDITION_BILLING,
            {
                let mut __s = ::std::vec::Vec::new();
                __s
            },
        )
        .await
    {
        ::core::result::Result::Ok(g) => g,
        ::core::result::Result::Err(e) => {
            return ::air_tight::__private::axum::response::IntoResponse::into_response(
                e,
            );
        }
    };
    #[allow(unused_variables)]
    let granted = __air_tight_granted;
    let __air_tight_out: Response = async move { StatusCode::OK.into_response() }.await;
    ::air_tight::__private::axum::response::IntoResponse::into_response(__air_tight_out)
}
///
///Requires `any(Billing(Read), !Workspace(Delete))`.
///
///Route with `air_tight::routing::get(billing)` (or `post`, `patch`, ...). The handler body lives in a hidden function; this one only hands it to the typed router.
fn billing<__S, __M>() -> ::air_tight::Guarded<
    impl ::air_tight::__private::axum::handler::Handler<
        (__M, State<Client>, Extension<User>),
        __S,
    >,
>
where
    __S: ::core::clone::Clone + ::core::marker::Send + ::core::marker::Sync + 'static,
    State<Client>: ::air_tight::__private::axum::extract::FromRequestParts<__S>,
    Extension<User>: ::air_tight::__private::axum::extract::FromRequest<__S, __M>,
{
    ::air_tight::Guarded::__new(__air_tight_handler_billing)
}
#[doc(hidden)]
#[allow(non_camel_case_types)]
struct __Protected_billing;
impl ::air_tight::router::Protected for __Protected_billing {
    const HANDLER: &'static str = ::core::concat!(
        ::core::module_path!(), "::", "billing"
    );
    const CONDITION: &'static str = "any(Billing(Read), !Workspace(Delete))";
}
::air_tight::__private::inventory::submit! {
    ::air_tight::registry::HandlerEntry { module_path : ::core::module_path!(), name :
    "billing", kind : ::air_tight::registry::HandlerKind::Protected, condition :
    "any(Billing(Read), !Workspace(Delete))", keys : || { __AIR_TIGHT_CONDITION_BILLING
    .leaves().into_iter().map(| p | ::air_tight::PermissionCategory::key(p).to_string())
    .collect() }, resource_kinds : || { let mut kinds : ::std::vec::Vec <
    ::std::string::String > = __AIR_TIGHT_CONDITION_BILLING.leaves().into_iter()
    .filter_map(| p | ::air_tight::PermissionCategory::resource_kind(p)).map(| k | k
    .to_string()).collect(); kinds.sort(); kinds.dedup(); kinds }, }
}
