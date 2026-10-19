use std::fmt;

use crate::Error;

/// A validated role name: non-empty and without leading or trailing whitespace.
///
/// [`RoleName::new`] trims its input, so `"  admin "` becomes `admin`; a name
/// that is empty after trimming is rejected with `Error::Catalog`. Names are
/// unique in the database.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct RoleName(String);

impl RoleName {
    /// Validates and wraps a role name.
    pub fn new(name: &str) -> Result<Self, Error> {
        let name = name.trim();
        if name.is_empty() {
            return Err(Error::Catalog("role name must not be empty".to_owned()));
        }
        Ok(RoleName(name.to_owned()))
    }

    /// Returns the name as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for RoleName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trims_and_rejects_empty() {
        assert_eq!(RoleName::new("  admin ").unwrap().as_str(), "admin");
        assert!(RoleName::new("").is_err());
        assert!(RoleName::new("   ").is_err());
    }
}
