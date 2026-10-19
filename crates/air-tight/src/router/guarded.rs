/// A handler wrapped by `#[air_tight::permission]`.
///
/// The typed router accepts only `Guarded` and [`Public`](super::Public)
/// handlers. Construct it through the macro, never by hand: a hand-built
/// `Guarded` is not in the registry and makes
/// [`AirTight::audit_routes`](crate::AirTight::audit_routes) panic.
#[derive(Clone, Copy, Debug)]
pub struct Guarded<H>(H);

impl<H> Guarded<H> {
    /// Used by generated code. Not public API.
    #[doc(hidden)]
    pub fn __new(handler: H) -> Self {
        Guarded(handler)
    }

    pub(crate) fn into_inner(self) -> H {
        self.0
    }
}
