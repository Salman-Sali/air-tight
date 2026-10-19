use std::borrow::Cow;
use std::fmt;

use serde::{Deserialize, Serialize};

/// Key identifying a permission, e.g. `Workspace::Update`, or a wildcard grant key.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PermissionKey(Cow<'static, str>);

impl PermissionKey {
    /// Builds a key from a static string, usable in `const` contexts.
    pub const fn from_static(s: &'static str) -> Self {
        PermissionKey(Cow::Borrowed(s))
    }

    /// Builds a key from any string-like value.
    pub fn new(s: impl Into<String>) -> Self {
        PermissionKey(Cow::Owned(s.into()))
    }

    /// Returns the key as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for PermissionKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const K: PermissionKey = PermissionKey::from_static("A::B");

    #[test]
    fn const_and_owned_are_equal() {
        assert_eq!(K, PermissionKey::new("A::B"));
        assert_eq!(K.as_str(), "A::B");
        assert_eq!(K.to_string(), "A::B");
    }
}
