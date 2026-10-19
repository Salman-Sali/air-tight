use std::collections::{HashMap, HashSet};
use std::mem::size_of;
use std::sync::Arc;

use air_tight_core::{PermissionKey, ResourceKindKey};

use crate::{Interner, KeyId, KindId};

/// The flattened grants of one user: global keys and keys scoped to a resource.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct GrantSet {
    global: HashSet<KeyId>,
    scoped: HashMap<(KindId, Arc<str>), HashSet<KeyId>>,
}

impl GrantSet {
    /// Creates an empty set.
    pub fn new() -> Self {
        Self::default()
    }

    /// Keys granted globally.
    pub fn global(&self) -> &HashSet<KeyId> {
        &self.global
    }

    /// Keys granted within a specific `(kind, resource id)` scope.
    pub fn scoped(&self) -> &HashMap<(KindId, Arc<str>), HashSet<KeyId>> {
        &self.scoped
    }

    /// Whether the user holds no grants at all.
    pub fn is_empty(&self) -> bool {
        self.global.is_empty() && self.scoped.is_empty()
    }

    /// Adds a global grant.
    pub(crate) fn insert_global(&mut self, key: KeyId) {
        self.global.insert(key);
    }

    /// Adds a grant within a scope.
    pub(crate) fn insert_scoped(&mut self, key: KeyId, kind: KindId, id: &str) {
        self.scoped
            .entry((kind, Arc::from(id)))
            .or_default()
            .insert(key);
    }

    /// Whether `key` is allowed, optionally within `scope`.
    ///
    /// A grant matches when it equals the key, is `<Category>::*`, or is `*`.
    /// With a scope, a global grant or a grant for that exact `(kind, id)` passes.
    /// Keys never interned cannot be held and yield `false` without being interned.
    pub fn allows(&self, key: &PermissionKey, scope: Option<(&ResourceKindKey, &str)>) -> bool {
        let interner = Interner::global();
        let category_wildcard = key
            .as_str()
            .split_once("::")
            .map(|(category, _)| format!("{category}::*"));
        let candidates: [Option<KeyId>; 3] = [
            interner.key_id(key.as_str()),
            category_wildcard.and_then(|w| interner.key_id(&w)),
            interner.key_id("*"),
        ];
        let held = |set: &HashSet<KeyId>| candidates.iter().flatten().any(|k| set.contains(k));
        if held(&self.global) {
            return true;
        }
        let Some((kind, id)) = scope else {
            return false;
        };
        let Some(kind_id) = interner.kind_id(kind.as_str()) else {
            return false;
        };
        self.scoped.get(&(kind_id, Arc::from(id))).is_some_and(held)
    }

    /// Approximate heap size in bytes, for cache weighing.
    pub fn approx_bytes(&self) -> u32 {
        let key = size_of::<KeyId>();
        let mut total = size_of::<Self>() + self.global.len() * (key + 1);
        for ((_, id), keys) in &self.scoped {
            total += size_of::<KindId>() + size_of::<Arc<str>>() + id.len() + 32;
            total += keys.len() * (key + 1);
        }
        u32::try_from(total).unwrap_or(u32::MAX)
    }
}
