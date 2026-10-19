use std::sync::OnceLock;

use crate::intern_table::InternTable;
use crate::{Error, KeyId, KindId};

/// The process-wide interner with one id space for keys and one for kinds.
#[derive(Default)]
pub struct Interner {
    keys: InternTable,
    kinds: InternTable,
}

static GLOBAL: OnceLock<Interner> = OnceLock::new();

impl Interner {
    /// Returns the process-wide interner.
    pub fn global() -> &'static Interner {
        GLOBAL.get_or_init(Interner::default)
    }

    /// Interns a permission or grant key (wildcards included), returning its stable id.
    pub fn intern_key(&self, key: &str) -> Result<KeyId, Error> {
        self.keys.intern(key, "permission keys").map(KeyId)
    }

    /// Looks up a key id without interning. `None` means the key was never seen.
    pub fn key_id(&self, key: &str) -> Option<KeyId> {
        self.keys.get(key).map(KeyId)
    }

    /// Interns a resource kind key, returning its stable id.
    pub fn intern_kind(&self, kind: &str) -> Result<KindId, Error> {
        self.kinds.intern(kind, "resource kinds").map(KindId)
    }

    /// Looks up a kind id without interning. `None` means the kind was never seen.
    pub fn kind_id(&self, kind: &str) -> Option<KindId> {
        self.kinds.get(kind).map(KindId)
    }
}
