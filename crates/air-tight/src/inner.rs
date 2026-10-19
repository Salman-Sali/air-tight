use std::sync::Arc;

use diesel::PgConnection;
use diesel::prelude::*;

use crate::generations::Generations;
use crate::invalidation::Invalidation;
use crate::schema::user_roles;
use crate::{
    CatalogSyncReport, Error, GrantCache, GrantSet, NotifyChannel, PgPool, RoleId, load_grant_set,
};

/// Catalog sync entry point monomorphised over the app's types at build time.
pub(crate) type SyncFn = fn(&mut PgConnection) -> Result<CatalogSyncReport, Error>;

/// Shared state of an [`crate::AirTight`] client.
///
/// Everything that does not depend on the app's types lives here, so the
/// listener thread can work on it through a `Weak` reference.
pub(crate) struct Inner {
    pub(crate) pool: PgPool,
    pub(crate) cache: Arc<dyn GrantCache>,
    pub(crate) channel: NotifyChannel,
    pub(crate) sync: SyncFn,
    pub(crate) generations: Generations,
}

impl Inner {
    /// Loads the grant set of `user_id` and caches it unless an invalidation
    /// raced with the load.
    ///
    /// The caller always gets the freshly loaded set. The cache insert is
    /// guarded by [`Generations`]: invalidations bump a generation before they
    /// clear the cache, so checking after the insert and clearing the entry on
    /// a mismatch leaves no window in which a stale set can survive.
    pub(crate) fn load_and_cache(&self, user_id: &str) -> Result<Arc<GrantSet>, Error> {
        let snapshot = self.generations.snapshot(user_id);
        let mut conn = self.pool.get()?;
        let set = Arc::new(load_grant_set(&mut conn, user_id)?);
        drop(conn);
        if self.generations.is_current(user_id, &snapshot) {
            self.cache.insert(user_id.to_owned(), Arc::clone(&set));
            if !self.generations.is_current(user_id, &snapshot) {
                self.cache.invalidate(user_id);
            }
        }
        Ok(set)
    }

    /// Drops the cached grants of `user_id` and stales in-flight loads.
    pub(crate) fn invalidate_user(&self, user_id: &str) {
        self.generations.bump_user(user_id);
        self.cache.invalidate(user_id);
    }

    /// Drops every cached grant set and stales in-flight loads.
    pub(crate) fn invalidate_all(&self) {
        self.generations.bump_all();
        self.cache.invalidate_all();
    }

    /// Invalidates exactly the users that currently hold `role`.
    pub(crate) fn invalidate_role(
        &self,
        conn: &mut PgConnection,
        role: RoleId,
    ) -> Result<(), Error> {
        let users: Vec<String> = user_roles::table
            .filter(user_roles::role_id.eq(role.as_uuid()))
            .select(user_roles::user_id)
            .distinct()
            .load(conn)?;
        for user in &users {
            self.invalidate_user(user);
        }
        Ok(())
    }

    /// Applies an invalidation to the local cache; never fails.
    ///
    /// A failed role lookup falls back to dropping everything, which is always
    /// safe.
    pub(crate) fn apply(&self, conn: &mut PgConnection, invalidation: &Invalidation) {
        match invalidation {
            Invalidation::User(id) => self.invalidate_user(id),
            Invalidation::All => self.invalidate_all(),
            Invalidation::Role(role) => {
                if let Err(err) = self.invalidate_role(conn, *role) {
                    tracing::error!(error = %err, role = %role, "role lookup failed, invalidating everything");
                    self.invalidate_all();
                }
            }
        }
    }

    /// Runs `work` in one transaction, sends the invalidations it returns as
    /// NOTIFY inside that transaction, and after the commit applies them to the
    /// local cache too.
    ///
    /// Postgres delivers the notifications on commit, so other instances never
    /// see them before the data is visible. The direct local invalidation means
    /// correctness does not depend on the listener thread.
    pub(crate) fn mutate<T>(
        &self,
        work: impl FnOnce(&mut PgConnection) -> Result<(T, Vec<Invalidation>), Error>,
    ) -> Result<T, Error> {
        let mut conn = self.pool.get()?;
        let (value, invalidations) = conn.transaction::<_, Error, _>(|conn| {
            let (value, invalidations) = work(conn)?;
            for invalidation in &invalidations {
                invalidation.notify(conn, &self.channel)?;
            }
            Ok((value, invalidations))
        })?;
        for invalidation in &invalidations {
            self.apply(&mut conn, invalidation);
        }
        Ok(value)
    }
}
