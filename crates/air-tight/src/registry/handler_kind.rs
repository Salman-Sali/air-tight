/// Whether a registered handler is protected by a condition or public.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum HandlerKind {
    /// Declared with `#[air_tight::permission(...)]`.
    Protected,
    /// Declared with `#[air_tight::unrestricted]`.
    Public,
}
