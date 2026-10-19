//! The example's fake authentication: a user id in the `X-User-Id` header.

mod authenticate;
mod authenticated_user;
mod user_id;

pub use authenticate::authenticate;
pub use authenticated_user::AuthenticatedUser;
pub use user_id::UserId;
