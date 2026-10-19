//! Permission and resource catalogs for the example app.

mod billing_action;
mod invoice_action;
mod my_perms;
mod my_resources;
mod workspace_action;

pub use billing_action::BillingAction;
pub use invoice_action::InvoiceAction;
pub use my_perms::MyPerms;
pub use my_resources::{MyResources, my_resources_kinds};
pub use workspace_action::WorkspaceAction;
