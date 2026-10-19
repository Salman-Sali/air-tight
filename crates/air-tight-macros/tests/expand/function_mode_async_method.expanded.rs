///
///Requires `Workspace(Update)`. Parameters injected by `#[air_tight::permission(kind = fn)]`, in front of the declared ones: `__air_tight: &AirTight<Perms, User>`, `__air_tight_user: &User`, `__air_tight_scopes: Vec<ResourceScope>`.
pub async fn rename(
    &self,
    __air_tight: &::air_tight::AirTight<
        crate::__air_tight::Perms,
        crate::__air_tight::User,
    >,
    __air_tight_user: &crate::__air_tight::User,
    __air_tight_scopes: ::std::vec::Vec<::air_tight::ResourceScope>,
    ws: WorkspaceRef,
    name: String,
) -> Result<(), AppError> {
    #[allow(non_upper_case_globals)]
    static __AIR_TIGHT_CONDITION: ::air_tight::Condition<crate::__air_tight::Perms> = ::air_tight::Condition::Leaf(
        match <crate::__air_tight::Perms>::__air_tight_lookup("Workspace", "Update") {
            ::core::option::Option::Some(p) => p,
            ::core::option::Option::None => {
                ::core::panic!(
                    "air_tight: `Workspace(Update)` is not a permission: no category variant `Workspace` wrapping an action variant `Update`"
                )
            }
        },
    );
    let __air_tight_granted = __air_tight
        .authorize_async(
            __air_tight_user,
            &__AIR_TIGHT_CONDITION,
            {
                let mut __s = __air_tight_scopes;
                __s.extend(::air_tight::IdentifyResources::resource_scopes(&ws));
                __s
            },
        )
        .await?;
    #[allow(unused_variables)]
    let granted = __air_tight_granted;
    self.repo.rename(ws, name)
}
