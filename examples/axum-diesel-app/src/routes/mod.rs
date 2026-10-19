//! Route handlers, one per file.

mod admin_assign;
mod admin_grant;
mod admin_roles;
mod billing;
mod billing_readonly;
mod create_invoice;
mod get_invoice;
mod get_workspace;
mod health;
mod list_workspaces;
mod list_workspaces_sql;
mod move_invoice;
mod update_workspace;

pub use admin_assign::admin_assign;
pub use admin_grant::admin_grant;
pub use admin_roles::admin_roles;
pub use billing::billing;
pub use billing_readonly::billing_readonly;
pub use create_invoice::create_invoice;
pub use get_invoice::get_invoice;
pub use get_workspace::get_workspace;
pub use health::health;
pub use list_workspaces::list_workspaces;
pub use list_workspaces_sql::list_workspaces_sql;
pub use move_invoice::move_invoice;
pub use update_workspace::update_workspace;
