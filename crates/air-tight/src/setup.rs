/// Declares the app's permission, principal and resource kind types for the
/// attribute macros. Invoke it exactly once, in the crate root (`main.rs` or
/// `lib.rs`):
///
/// ```ignore
/// air_tight::setup!(perms = MyPerms, user = AuthenticatedUser, resources = MyResources);
/// ```
///
/// Paths are resolved from the crate root. It emits a hidden module
/// `__air_tight` with the type aliases `Perms`, `User` and `Resources`, which
/// `#[air_tight::permission]` refers to as `crate::__air_tight::*`. If you
/// forget it, the attribute fails with "could not find `__air_tight` in the
/// crate root": add the `setup!` call.
#[macro_export]
macro_rules! setup {
    (perms = $perms:ty, user = $user:ty, resources = $resources:ty $(,)?) => {
        #[doc(hidden)]
        type __AirTightSetupPerms = $perms;
        #[doc(hidden)]
        type __AirTightSetupUser = $user;
        #[doc(hidden)]
        type __AirTightSetupResources = $resources;

        /// Types registered with `air_tight::setup!`.
        #[doc(hidden)]
        pub(crate) mod __air_tight {
            /// The app's permission enum.
            pub(crate) type Perms = super::__AirTightSetupPerms;
            /// The app's principal type.
            pub(crate) type User = super::__AirTightSetupUser;
            /// The app's resource kind enum.
            pub(crate) type Resources = super::__AirTightSetupResources;

            const _: () = {
                fn __air_tight_check<P, U, R>()
                where
                    P: $crate::PermissionCategory,
                    U: $crate::Principal,
                    R: $crate::ResourceKind,
                {
                }
                let _ = __air_tight_check::<Perms, User, Resources>;
            };
        }
    };
}
