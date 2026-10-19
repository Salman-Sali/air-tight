/// Typestate marker of a [`crate::Builder`] that has no resource kinds yet.
///
/// It deliberately does not implement `ResourceKind`, so `build()` is
/// unavailable until `.resource_kinds::<R>()` is called.
#[derive(Clone, Copy, Debug)]
pub struct NoKinds;
