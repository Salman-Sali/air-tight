#[doc(hidden)]
async fn __air_tight_handler_health() -> &'static str {
    "ok"
}
///
///Public: no permission required.
///
///Route with `air_tight::routing::get(health)` (or `post`, `patch`, ...). The handler body lives in a hidden function; this one only hands it to the typed router.
pub fn health<__S>() -> ::air_tight::Public<
    impl ::air_tight::__private::axum::handler::Handler<((),), __S>,
>
where
    __S: ::core::clone::Clone + ::core::marker::Send + ::core::marker::Sync + 'static,
{
    ::air_tight::Public::__new(__air_tight_handler_health)
}
::air_tight::__private::inventory::submit! {
    ::air_tight::registry::HandlerEntry { module_path : ::core::module_path!(), name :
    "health", kind : ::air_tight::registry::HandlerKind::Public, condition : "", keys :
    ::std::vec::Vec::new, resource_kinds : ::std::vec::Vec::new, }
}
