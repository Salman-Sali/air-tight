//! Re-exports used by macro-generated code. Not public API.

pub use axum;
pub use inventory;

/// Stand-in value for a never-called helper that keeps a handler's declared
/// `impl Trait` return type in the generated code.
pub fn unreachable_value<T>() -> T {
    unreachable!("air_tight: generated helper is never called")
}
