/// An authenticated actor, owned by the application.
pub trait Principal: Clone + Send + Sync + 'static {
    /// Stable textual id of the principal. The crate never parses it.
    fn air_tight_id(&self) -> String;
}
