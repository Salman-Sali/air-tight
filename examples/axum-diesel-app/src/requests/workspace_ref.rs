use air_tight::IdentifyResources;
use uuid::Uuid;

use crate::authz::my_resources_kinds::Workspace;

/// A reference to one workspace, handed to protected service functions so they
/// can name the resource they act on.
#[derive(Clone, Copy, Debug, IdentifyResources)]
pub struct WorkspaceRef {
    /// The workspace id.
    #[air_tight(resource = Workspace)]
    pub id: Uuid,
}
