use std::borrow::Cow;
use std::fmt;

use serde::{Deserialize, Serialize};

/// Key identifying a resource kind, e.g. `Workspace`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ResourceKindKey(Cow<'static, str>);

impl ResourceKindKey {
    /// Builds a key from a static string, usable in `const` contexts.
    pub const fn from_static(s: &'static str) -> Self {
        ResourceKindKey(Cow::Borrowed(s))
    }

    /// Builds a key from any string-like value.
    pub fn new(s: impl Into<String>) -> Self {
        ResourceKindKey(Cow::Owned(s.into()))
    }

    /// Returns the key as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ResourceKindKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const K: ResourceKindKey = ResourceKindKey::from_static("Invoice");

    #[test]
    fn const_and_owned_are_equal() {
        assert_eq!(K, ResourceKindKey::new("Invoice"));
        assert_eq!(K.as_str(), "Invoice");
        assert_eq!(K.to_string(), "Invoice");
    }
}
