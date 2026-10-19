use crate::GrantEntry;

/// Everything a user can do, as read from the database (not the cache).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct GrantReport {
    /// Grants without a scope.
    pub global: Vec<GrantEntry>,
    /// Grants limited to one resource.
    pub scoped: Vec<GrantEntry>,
}
