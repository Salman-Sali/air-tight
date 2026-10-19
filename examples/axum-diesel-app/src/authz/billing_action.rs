use air_tight::Permission;

/// Global billing actions.
#[derive(Permission)]
pub enum BillingAction {
    /// View billing.
    Read,
    /// Manage billing (stands in for an admin permission).
    Manage,
}
