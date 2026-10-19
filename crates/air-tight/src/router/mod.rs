//! The typed router: only handlers processed by `#[air_tight::permission]`
//! or `#[air_tight::unrestricted]` can be routed.

mod guarded;
mod method_router;
mod protected;
mod public;
mod route_record;
mod route_target;
mod typed_router;

pub use guarded::Guarded;
pub use method_router::MethodRouter;
pub use protected::Protected;
pub use public::Public;
pub use route_record::RouteRecord;
pub use route_target::RouteTarget;
pub use typed_router::Router;
