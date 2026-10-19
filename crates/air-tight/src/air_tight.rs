use std::marker::PhantomData;
use std::sync::Arc;

use air_tight_core::{Condition, PermissionCategory, Principal, ResourceScope};
use tokio::runtime::{Handle, RuntimeFlavor};

use crate::builder::Builder;
use crate::evaluate::ensure;
use crate::inner::Inner;
use crate::{CatalogSyncReport, Error, GrantSet, Granted, NoKinds, PgPool};

/// The authorization client: a database pool, a grant cache and the check API.
///
/// Cheap to clone (it is an `Arc` inside) and `Send + Sync` whatever `U` is.
/// Create it once at startup with [`AirTight::builder`].
///
/// # Using it as axum state
///
/// Because it is `Clone`, `State<AirTight<MyPerms, User>>` works directly when
/// the client is the whole router state. When it is one field of a larger app
/// state, implement `FromRef` so handlers can still extract it:
///
/// ```ignore
/// #[derive(Clone)]
/// struct AppState {
///     air_tight: AirTight<MyPerms, User>,
///     // ...
/// }
///
/// impl axum::extract::FromRef<AppState> for AirTight<MyPerms, User> {
///     fn from_ref(state: &AppState) -> Self {
///         state.air_tight.clone()
///     }
/// }
/// ```
///
/// Generic code cannot provide a blanket `FromRef` impl for every app state
/// (the orphan rules forbid it), so the app writes this impl itself, or uses
/// `#[derive(FromRef)]` from axum (enable axum's `macros` feature).
///
/// # Authentication
///
/// A request without an authenticated principal is not this layer's concern:
/// answering 401 is the job of the extractor that supplies the `&U` (added in
/// a later phase). `authorize` only ever sees a principal.
pub struct AirTight<P: PermissionCategory, U: Principal> {
    pub(crate) inner: Arc<Inner>,
    _types: PhantomData<fn() -> (P, U)>,
}

impl<P: PermissionCategory, U: Principal> Clone for AirTight<P, U> {
    fn clone(&self) -> Self {
        AirTight {
            inner: Arc::clone(&self.inner),
            _types: PhantomData,
        }
    }
}

impl<P: PermissionCategory, U: Principal> AirTight<P, U> {
    /// Starts building a client on `pool`.
    ///
    /// Call `.resource_kinds::<R>()` before `.build()`.
    pub fn builder(pool: PgPool) -> Builder<P, U, NoKinds> {
        Builder::new(pool)
    }

    pub(crate) fn from_inner(inner: Arc<Inner>) -> Self {
        AirTight {
            inner,
            _types: PhantomData,
        }
    }

    /// The embedded migrations as `(version, sql)` pairs, for self-managed setups.
    pub fn migration_sql() -> &'static [(&'static str, &'static str)] {
        crate::migration_sql()
    }

    /// Upserts the code catalog into the database again.
    ///
    /// `build()` already does this once; call it to re-sync on demand.
    pub fn sync_catalog(&self) -> Result<CatalogSyncReport, Error> {
        let mut conn = self.inner.pool.get()?;
        (self.inner.sync)(&mut conn)
    }

    /// Returns the grant set of `user_id`, loading and caching it on a miss.
    ///
    /// Blocks on the database on a miss; async code should prefer
    /// [`AirTight::authorize_async`].
    pub fn grant_set(&self, user_id: &str) -> Result<Arc<GrantSet>, Error> {
        match self.inner.cache.get(user_id) {
            Some(set) => Ok(set),
            None => self.inner.load_and_cache(user_id),
        }
    }

    /// Checks `cond` for `user_id` against the request's resource `scopes`.
    ///
    /// Fails with `Error::Forbidden` carrying the missing permission keys.
    pub fn check(
        &self,
        user_id: &str,
        cond: &Condition<P>,
        scopes: &[ResourceScope],
    ) -> Result<(), Error> {
        let set = self.grant_set(user_id)?;
        ensure(&set, cond, scopes)
    }

    /// Checks `cond` for `user` and returns a [`Granted`] holding the snapshot.
    ///
    /// Blocks on the database on a cache miss.
    pub fn authorize(
        &self,
        user: &U,
        cond: &Condition<P>,
        scopes: Vec<ResourceScope>,
    ) -> Result<Granted<P>, Error> {
        let user_id = user.air_tight_id();
        let set = self.grant_set(&user_id)?;
        Self::finish(&user_id, set, cond, scopes)
    }

    /// Async variant of [`AirTight::authorize`] that never stalls the executor.
    ///
    /// A cache hit is evaluated inline. On a miss the load runs through
    /// `block_in_place` on a multi-thread runtime, through `spawn_blocking` on
    /// any other runtime, and synchronously when called outside a runtime. The
    /// returned future is `Send`.
    pub async fn authorize_async(
        &self,
        user: &U,
        cond: &Condition<P>,
        scopes: Vec<ResourceScope>,
    ) -> Result<Granted<P>, Error> {
        let user_id = user.air_tight_id();
        let set = match self.inner.cache.get(&user_id) {
            Some(set) => set,
            None => self.load_async(user_id.clone()).await?,
        };
        Self::finish(&user_id, set, cond, scopes)
    }

    /// Drops the cached grants of `user_id`; the next check reloads them.
    pub fn invalidate_user(&self, user_id: &str) {
        self.inner.invalidate_user(user_id);
    }

    /// Drops every cached grant set.
    pub fn invalidate_all(&self) {
        self.inner.invalidate_all();
    }

    fn finish(
        user_id: &str,
        set: Arc<GrantSet>,
        cond: &Condition<P>,
        scopes: Vec<ResourceScope>,
    ) -> Result<Granted<P>, Error> {
        ensure(&set, cond, &scopes)?;
        Ok(Granted::new(user_id, scopes, set))
    }

    async fn load_async(&self, user_id: String) -> Result<Arc<GrantSet>, Error> {
        match Handle::try_current() {
            Ok(handle) if handle.runtime_flavor() == RuntimeFlavor::MultiThread => {
                tokio::task::block_in_place(|| self.inner.load_and_cache(&user_id))
            }
            Ok(_) => {
                let this = self.clone();
                tokio::task::spawn_blocking(move || this.inner.load_and_cache(&user_id))
                    .await
                    .map_err(|err| {
                        tracing::error!(error = %err, "grant loading task failed");
                        Error::Pool
                    })?
            }
            Err(_) => self.inner.load_and_cache(&user_id),
        }
    }
}
