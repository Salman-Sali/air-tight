use air_tight::IdentifyResources;
use serde::Deserialize;
use uuid::Uuid;

use crate::authz::my_resources_kinds::Workspace;

/// Body of `POST /invoices`.
#[derive(Debug, Deserialize, IdentifyResources)]
pub struct CreateInvoice {
    /// The workspace to create the invoice in.
    #[air_tight(resource = Workspace)]
    pub workspace_id: Uuid,
    /// The amount in cents.
    pub amount: i64,
}
