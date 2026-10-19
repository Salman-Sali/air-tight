#[allow(non_upper_case_globals)]
static __AIR_TIGHT_CONDITION_GET_WORKSPACE: ::air_tight::Condition<
    crate::__air_tight::Perms,
> = ::air_tight::Condition::Any(
    &[
        ::air_tight::Condition::Leaf(
            match <crate::__air_tight::Perms>::__air_tight_lookup("Workspace", "Read") {
                ::core::option::Option::Some(p) => p,
                ::core::option::Option::None => {
                    ::core::panic!(
                        "air_tight: `Workspace(Read)` is not a permission: no category variant `Workspace` wrapping an action variant `Read`"
                    )
                }
            },
        ),
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
    ],
);
#[doc(hidden)]
async fn __air_tight_handler_get_workspace(
    ::air_tight::__private::axum::extract::State(
        __air_tight,
    ): ::air_tight::__private::axum::extract::State<
        ::air_tight::AirTight<crate::__air_tight::Perms, crate::__air_tight::User>,
    >,
    ::air_tight::CurrentUser(
        __air_tight_user,
    ): ::air_tight::CurrentUser<crate::__air_tight::User>,
    __air_tight_auto_0: AirTightPath<Uuid, Workspace>,
) -> ::air_tight::__private::axum::response::Response {
    #[allow(dead_code)]
    fn __air_tight_declared_return() -> impl IntoResponse {
        ::air_tight::__private::unreachable_value::<::core::convert::Infallible>()
    }
    let __air_tight_granted = match __air_tight
        .authorize_async(
            &__air_tight_user,
            &__AIR_TIGHT_CONDITION_GET_WORKSPACE,
            {
                let mut __s = ::std::vec::Vec::new();
                __s.extend(
                    ::air_tight::IdentifyResources::resource_scopes(&__air_tight_auto_0),
                );
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
    let AirTightPath(Path(id)) = __air_tight_auto_0;
    #[allow(unused_variables)]
    let granted = __air_tight_granted;
    let __air_tight_out = async move { Json(id) }.await;
    ::air_tight::__private::axum::response::IntoResponse::into_response(__air_tight_out)
}
///
///Requires `any(Workspace(Read), Workspace(Update))`.
///
///Route with `air_tight::routing::get(get_workspace)` (or `post`, `patch`, ...). The handler body lives in a hidden function; this one only hands it to the typed router.
fn get_workspace<__S, __M>() -> ::air_tight::Guarded<
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
            AirTightPath<Uuid, Workspace>,
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
    AirTightPath<
        Uuid,
        Workspace,
    >: ::air_tight::__private::axum::extract::FromRequest<__S, __M>,
{
    ::air_tight::Guarded::__new(__air_tight_handler_get_workspace)
}
#[doc(hidden)]
#[allow(non_camel_case_types)]
struct __Protected_get_workspace;
impl ::air_tight::router::Protected for __Protected_get_workspace {
    const HANDLER: &'static str = ::core::concat!(
        ::core::module_path!(), "::", "get_workspace"
    );
    const CONDITION: &'static str = "any(Workspace(Read), Workspace(Update))";
}
::air_tight::__private::inventory::submit! {
    ::air_tight::registry::HandlerEntry { module_path : ::core::module_path!(), name :
    "get_workspace", kind : ::air_tight::registry::HandlerKind::Protected, condition :
    "any(Workspace(Read), Workspace(Update))", keys : || {
    __AIR_TIGHT_CONDITION_GET_WORKSPACE.leaves().into_iter().map(| p |
    ::air_tight::PermissionCategory::key(p).to_string()).collect() }, resource_kinds : ||
    { let mut kinds : ::std::vec::Vec < ::std::string::String > =
    __AIR_TIGHT_CONDITION_GET_WORKSPACE.leaves().into_iter().filter_map(| p |
    ::air_tight::PermissionCategory::resource_kind(p)).map(| k | k.to_string())
    .collect(); kinds.sort(); kinds.dedup(); kinds }, }
}
