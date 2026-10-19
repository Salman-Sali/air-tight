use air_tight::Permission;

/// Actions on an invoice.
#[derive(Permission)]
pub enum InvoiceAction {
    /// View an invoice.
    Read,
    /// Create an invoice.
    Create,
    /// Edit or move an invoice.
    Update,
}
