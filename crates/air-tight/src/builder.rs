use std::marker::PhantomData;
use std::sync::Arc;
use std::time::Duration;

use air_tight_core::{PermissionCategory, Principal, ResourceKind};

use crate::inner::Inner;
use crate::{
    AirTight, Error, GrantCache, MokaGrantCache, NoKinds, NotifyChannel, PgPool, run_migrations,
    sync_catalog,
};

const DEFAULT_IDLE_TTL: Duration = Duration::from_secs(10 * 60);
const DEFAULT_MAX_BYTES: u64 = 256 * 1024 * 1024;

/// Zero-sized marker keeping the builder's types without owning them.
type TypeMarker<P, U, R> = PhantomData<fn() -> (P, U, R)>;

/// Builder of an [`AirTight`] client.
///
/// `R` is the app's `ResourceKind` enum, or [`NoKinds`] until
/// [`Builder::resource_kinds`] is called. `build()` only exists once `R` is a
/// real `ResourceKind`, so forgetting `.resource_kinds::<R>()` is a compile
/// error rather than a runtime one:
///
/// ```compile_fail
/// # use air_tight::*;
/// # fn f<P: PermissionCategory, U: Principal>(pool: PgPool) {
/// let _ = AirTight::<P, U>::builder(pool).build();
/// # }
/// ```
pub struct Builder<P: PermissionCategory, U: Principal, R> {
    pool: PgPool,
    idle_ttl: Duration,
    max_bytes: u64,
    channel: Result<NotifyChannel, Error>,
    cache: Option<Arc<dyn GrantCache>>,
    _types: TypeMarker<P, U, R>,
}

impl<P: PermissionCategory, U: Principal> Builder<P, U, NoKinds> {
    pub(crate) fn new(pool: PgPool) -> Self {
        Builder {
            pool,
            idle_ttl: DEFAULT_IDLE_TTL,
            max_bytes: DEFAULT_MAX_BYTES,
            channel: Ok(NotifyChannel::default()),
            cache: None,
            _types: PhantomData,
        }
    }
}

impl<P: PermissionCategory, U: Principal, R> Builder<P, U, R> {
    /// Idle time-to-live of cached grant sets (default 10 minutes).
    ///
    /// Ignored when a custom cache is supplied with [`Builder::cache`].
    pub fn cache_idle_ttl(mut self, ttl: Duration) -> Self {
        self.idle_ttl = ttl;
        self
    }

    /// Maximum approximate size of the default cache in bytes (default 256 MiB).
    ///
    /// Ignored when a custom cache is supplied with [`Builder::cache`].
    pub fn cache_max_bytes(mut self, bytes: u64) -> Self {
        self.max_bytes = bytes;
        self
    }

    /// The LISTEN/NOTIFY channel name (default `air_tight_invalidate`).
    ///
    /// Must be a plain identifier, `[a-z_][a-z0-9_]*`. An invalid name makes
    /// `build()` fail with `Error::Notify`.
    pub fn notify_channel(mut self, name: &str) -> Self {
        self.channel = NotifyChannel::new(name);
        self
    }

    /// Replaces the default moka cache with a custom [`GrantCache`].
    pub fn cache(mut self, cache: Arc<dyn GrantCache>) -> Self {
        self.cache = Some(cache);
        self
    }

    /// Declares the app's resource kind enum, required before `build()`.
    pub fn resource_kinds<K: ResourceKind>(self) -> Builder<P, U, K> {
        Builder {
            pool: self.pool,
            idle_ttl: self.idle_ttl,
            max_bytes: self.max_bytes,
            channel: self.channel,
            cache: self.cache,
            _types: PhantomData,
        }
    }
}

impl<P: PermissionCategory, U: Principal, R: ResourceKind> Builder<P, U, R> {
    /// Migrates the database, syncs the permission catalog and returns the client.
    ///
    /// Only available after `.resource_kinds::<R>()`.
    pub fn build(self) -> Result<AirTight<P, U>, Error> {
        let channel = self.channel?;
        let cache = self
            .cache
            .unwrap_or_else(|| Arc::new(MokaGrantCache::new(self.idle_ttl, self.max_bytes)));
        {
            let mut conn = self.pool.get()?;
            run_migrations(&mut conn)?;
            sync_catalog::<P, R>(&mut conn)?;
        }
        let client = AirTight::from_inner(Arc::new(Inner {
            pool: self.pool,
            cache,
            channel,
            sync: sync_catalog::<P, R>,
        }));
        client.start_listener();
        Ok(client)
    }
}
