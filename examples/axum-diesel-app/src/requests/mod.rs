//! JSON request bodies.

mod assign_role;
mod create_invoice;
mod create_role;
mod grant_permission;
mod move_invoice;
mod rename_workspace;
mod scope_dto;
mod workspace_ref;

pub use assign_role::AssignRole;
pub use create_invoice::CreateInvoice;
pub use create_role::CreateRole;
pub use grant_permission::GrantPermission;
pub use move_invoice::MoveInvoice;
pub use rename_workspace::RenameWorkspace;
pub use scope_dto::ScopeDto;
pub use workspace_ref::WorkspaceRef;
