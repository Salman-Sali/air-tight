use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::atomic::{AtomicU64, Ordering};

/// Number of per-user generation counters; users share a counter by hash.
const STRIPES: usize = 64;

/// Generation counters guarding the cache against stale inserts.
///
/// A loader takes a [`Snapshot`] before reading the database and checks it
/// afterwards. Invalidations bump a counter before touching the cache, so a
/// loader that raced with an invalidation can always tell. Per-user counters
/// are striped by hash: two users may share a stripe, which only costs a
/// skipped cache insert, never a stale one.
pub(crate) struct Generations {
    global: AtomicU64,
    stripes: Box<[AtomicU64]>,
}

/// The generation values observed before a load started.
pub(crate) struct Snapshot {
    global: u64,
    stripe: u64,
}

impl Generations {
    pub(crate) fn new() -> Self {
        Generations {
            global: AtomicU64::new(0),
            stripes: (0..STRIPES).map(|_| AtomicU64::new(0)).collect(),
        }
    }

    fn stripe(&self, user_id: &str) -> &AtomicU64 {
        let mut hasher = DefaultHasher::new();
        user_id.hash(&mut hasher);
        // The modulo keeps the value below STRIPES, so the cast cannot truncate.
        &self.stripes[(hasher.finish() % STRIPES as u64) as usize]
    }

    /// Records the current generations for `user_id`.
    pub(crate) fn snapshot(&self, user_id: &str) -> Snapshot {
        Snapshot {
            global: self.global.load(Ordering::SeqCst),
            stripe: self.stripe(user_id).load(Ordering::SeqCst),
        }
    }

    /// Whether no invalidation affecting `user_id` happened since `snapshot`.
    pub(crate) fn is_current(&self, user_id: &str, snapshot: &Snapshot) -> bool {
        self.global.load(Ordering::SeqCst) == snapshot.global
            && self.stripe(user_id).load(Ordering::SeqCst) == snapshot.stripe
    }

    /// Marks every in-flight load of `user_id` as stale.
    pub(crate) fn bump_user(&self, user_id: &str) {
        self.stripe(user_id).fetch_add(1, Ordering::SeqCst);
    }

    /// Marks every in-flight load as stale.
    pub(crate) fn bump_all(&self) {
        self.global.fetch_add(1, Ordering::SeqCst);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bumps_invalidate_snapshots() {
        let gens = Generations::new();
        let snap = gens.snapshot("a");
        assert!(gens.is_current("a", &snap));
        gens.bump_user("a");
        assert!(!gens.is_current("a", &snap));
        let snap = gens.snapshot("a");
        gens.bump_all();
        assert!(!gens.is_current("a", &snap));
    }
}
