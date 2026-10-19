use std::fmt;
use std::marker::PhantomData;
use std::sync::Arc;

use air_tight_core::{Condition, PermissionCategory, ResourceScope};

use crate::evaluate::ensure;
use crate::{Error, GrantSet};

/// Proof that a request passed [`crate::AirTight::authorize`].
///
/// It holds the user id, the request's resource scopes and a snapshot of the
/// user's grants taken at authorization time, so further checks via
/// [`Granted::require`] are in-memory and consistent with the first one.
/// Only `AirTight::authorize` and `authorize_async` can construct it.
pub struct Granted<P: PermissionCategory> {
    user_id: Arc<str>,
    scopes: Vec<ResourceScope>,
    snapshot: Arc<GrantSet>,
    _perms: PhantomData<fn() -> P>,
}

impl<P: PermissionCategory> Granted<P> {
    pub(crate) fn new(user_id: &str, scopes: Vec<ResourceScope>, snapshot: Arc<GrantSet>) -> Self {
        Granted {
            user_id: Arc::from(user_id),
            scopes,
            snapshot,
            _perms: PhantomData,
        }
    }

    /// Re-checks `cond` in memory against the held snapshot and scopes.
    ///
    /// Returns `Error::Forbidden` with the missing keys on failure. Never
    /// touches the database or the cache.
    pub fn require(&self, cond: &Condition<P>) -> Result<(), Error> {
        ensure(&self.snapshot, cond, &self.scopes)
    }

    /// The id of the authorized user.
    pub fn user_id(&self) -> &str {
        &self.user_id
    }

    /// The resource scopes of the request this was authorized for.
    pub fn scopes(&self) -> &[ResourceScope] {
        &self.scopes
    }
}

impl<P: PermissionCategory> Clone for Granted<P> {
    fn clone(&self) -> Self {
        Granted {
            user_id: Arc::clone(&self.user_id),
            scopes: self.scopes.clone(),
            snapshot: Arc::clone(&self.snapshot),
            _perms: PhantomData,
        }
    }
}

impl<P: PermissionCategory> fmt::Debug for Granted<P> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Granted")
            .field("user_id", &self.user_id)
            .field("scopes", &self.scopes)
            .finish_non_exhaustive()
    }
}
