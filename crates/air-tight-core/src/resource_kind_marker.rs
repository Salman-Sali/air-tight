use crate::ResourceKindKey;

/// Type-level marker for a resource kind.
pub trait ResourceKindMarker {
    /// The key of the marked kind.
    fn key() -> ResourceKindKey;
}
