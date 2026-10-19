/// Implemented by the `__Protected_<fn>` marker struct that
/// `#[air_tight::permission]` emits next to each protected handler.
pub trait Protected {
    /// `module_path::fn_name` of the handler.
    const HANDLER: &'static str;
    /// The condition as written in the attribute.
    const CONDITION: &'static str;
}
