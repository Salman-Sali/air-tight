/// Interned id of a permission or grant key (including wildcards).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct KeyId(pub(crate) u16);

impl KeyId {
    /// Returns the raw numeric id.
    pub const fn get(self) -> u16 {
        self.0
    }
}
