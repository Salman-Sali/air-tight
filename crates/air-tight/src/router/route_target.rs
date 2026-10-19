use axum::handler::Handler;

use super::{Guarded, Public};

mod sealed {
    pub trait Sealed {}
    impl<H> Sealed for super::Guarded<H> {}
    impl<H> Sealed for super::Public<H> {}
}

/// What the typed routing functions accept: [`Guarded`] or [`Public`].
///
/// Sealed, so a plain axum handler can never satisfy it.
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not a guarded or public handler",
    label = "plain axum handlers cannot be routed by the air_tight router",
    note = "annotate the handler with `#[air_tight::permission(...)]` or `#[air_tight::unrestricted]`"
)]
pub trait RouteTarget<T, S>: sealed::Sealed {
    /// The underlying axum handler.
    type Handler: Handler<T, S>;
    /// Unwraps the axum handler.
    fn into_handler(self) -> Self::Handler;
}

impl<H: Handler<T, S>, T, S> RouteTarget<T, S> for Guarded<H> {
    type Handler = H;
    fn into_handler(self) -> H {
        self.into_inner()
    }
}

impl<H: Handler<T, S>, T, S> RouteTarget<T, S> for Public<H> {
    type Handler = H;
    fn into_handler(self) -> H {
        self.into_inner()
    }
}
