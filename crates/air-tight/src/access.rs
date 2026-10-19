/// Which resources of one kind a user may access with one permission.
///
/// Returned by [`crate::AirTight::accessible_ids`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Access {
    /// A global grant (or a wildcard) covers every resource of the kind.
    All,
    /// Only these resource ids (sorted, without duplicates) are covered.
    Some(Vec<String>),
    /// No resource of the kind is covered.
    None,
}
