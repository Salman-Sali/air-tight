/// A handler marked `#[air_tight::unrestricted]`: routable without a
/// permission check.
#[derive(Clone, Copy, Debug)]
pub struct Public<H>(H);

impl<H> Public<H> {
    /// Used by generated code. Not public API.
    #[doc(hidden)]
    pub fn __new(handler: H) -> Self {
        Public(handler)
    }

    pub(crate) fn into_inner(self) -> H {
        self.0
    }
}
