/// Interned id of a resource kind key.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct KindId(pub(crate) u16);

impl KindId {
    /// Returns the raw numeric id.
    pub const fn get(self) -> u16 {
        self.0
    }
}
