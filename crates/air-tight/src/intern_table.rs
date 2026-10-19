use std::collections::HashMap;
use std::sync::RwLock;

use crate::Error;

/// One id space: a thread-safe string to `u16` map with a read-mostly fast path.
#[derive(Default)]
pub(crate) struct InternTable {
    inner: RwLock<Inner>,
}

#[derive(Default)]
struct Inner {
    ids: HashMap<Box<str>, u16>,
    count: usize,
}

impl InternTable {
    /// Returns the id of `s` if it was interned, without inserting.
    pub(crate) fn get(&self, s: &str) -> Option<u16> {
        let inner = self.inner.read().unwrap_or_else(|e| e.into_inner());
        inner.ids.get(s).copied()
    }

    /// Returns the id of `s`, interning it if needed.
    pub(crate) fn intern(&self, s: &str, space: &str) -> Result<u16, Error> {
        if let Some(id) = self.get(s) {
            return Ok(id);
        }
        let mut inner = self.inner.write().unwrap_or_else(|e| e.into_inner());
        if let Some(id) = inner.ids.get(s) {
            return Ok(*id);
        }
        let id = u16::try_from(inner.count).map_err(|_| {
            Error::Catalog(format!("too many distinct {space} (limit {})", u16::MAX))
        })?;
        inner.ids.insert(s.into(), id);
        inner.count += 1;
        Ok(id)
    }

    #[cfg(test)]
    fn with_count(count: usize) -> Self {
        InternTable {
            inner: RwLock::new(Inner {
                ids: HashMap::new(),
                count,
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stable_and_lazy() {
        let t = InternTable::default();
        assert_eq!(t.get("a"), None);
        let a = t.intern("a", "keys").unwrap();
        let b = t.intern("b", "keys").unwrap();
        assert_ne!(a, b);
        assert_eq!(t.intern("a", "keys").unwrap(), a);
        assert_eq!(t.get("b"), Some(b));
    }

    #[test]
    fn overflow_is_an_error() {
        let t = InternTable::with_count(usize::from(u16::MAX));
        assert_eq!(t.intern("last", "keys").unwrap(), u16::MAX);
        assert!(matches!(t.intern("more", "keys"), Err(Error::Catalog(_))));
        assert_eq!(t.intern("last", "keys").unwrap(), u16::MAX);
    }
}
