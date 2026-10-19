use air_tight::{Condition, Error, Granted};
use serde::Serialize;

use crate::authz::{BillingAction, MyPerms};

/// What the caller may do with billing.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct BillingSummary {
    /// Whether the caller also holds `Billing::Manage`.
    pub can_manage: bool,
}

static MANAGE: Condition<MyPerms> = Condition::Leaf(MyPerms::Billing(BillingAction::Manage));

/// Re-checks `Billing(Manage)` in memory against the grants snapshot that the
/// request was authorized with. No database or cache access.
pub fn require_manage(granted: &Granted<MyPerms>) -> Result<(), Error> {
    granted.require(&MANAGE)
}

/// Summarizes the caller's billing rights from the `Granted` proof.
pub fn billing_summary(granted: &Granted<MyPerms>) -> BillingSummary {
    BillingSummary {
        can_manage: require_manage(granted).is_ok(),
    }
}
