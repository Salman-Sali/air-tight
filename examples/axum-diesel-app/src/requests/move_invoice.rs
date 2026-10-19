use air_tight::IdentifyResources;
use serde::Deserialize;
use uuid::Uuid;

use crate::authz::my_resources_kinds::{Invoice, Workspace};

/// Body of `POST /invoices/move`.
#[derive(Debug, Deserialize, IdentifyResources)]
pub struct MoveInvoice {
    /// The invoice to move.
    #[air_tight(resource = Invoice)]
    pub invoice_id: Uuid,
    /// Further invoices that move along with it. The caller needs
    /// `Invoice(Update)` on every one of them.
    #[serde(default)]
    #[air_tight(resource = Invoice)]
    pub related_invoice_ids: Vec<Uuid>,
    /// The workspace to move them into.
    #[air_tight(resource = Workspace)]
    pub target_workspace_id: Uuid,
}
