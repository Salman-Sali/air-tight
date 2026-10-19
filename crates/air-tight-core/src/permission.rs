use std::fmt::Debug;
use std::hash::Hash;

/// A layer of action variants (unit variants only), e.g. `Read`, `Update`.
pub trait Permission: Copy + Eq + Hash + Debug + Send + Sync + 'static {
    /// The action segment of the key (after `::`), respecting key overrides.
    fn action_name(&self) -> &'static str;
    /// Every action.
    fn all() -> &'static [Self];
}
