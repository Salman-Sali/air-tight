use air_tight::PermissionCategory;

use super::my_resources_kinds::{Invoice, Workspace};
use super::{BillingAction, InvoiceAction, WorkspaceAction};

/// Every permission in the example app.
#[derive(PermissionCategory)]
pub enum MyPerms {
    /// Workspace permissions, scoped by workspace id.
    #[air_tight(resource = Workspace)]
    Workspace(WorkspaceAction),
    /// Invoice permissions, scoped by invoice id.
    #[air_tight(resource = Invoice)]
    Invoice(InvoiceAction),
    /// Billing permissions, always global.
    Billing(BillingAction),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prints_all_keys() {
        let keys: Vec<String> = MyPerms::all().iter().map(|p| p.key().to_string()).collect();
        for k in &keys {
            println!("{k}");
        }
        assert_eq!(
            keys,
            [
                "Workspace::Read",
                "Workspace::Update",
                "Workspace::Delete",
                "Invoice::Read",
                "Invoice::Create",
                "Invoice::Update",
                "Billing::Read",
                "Billing::Manage",
            ]
        );
    }
}
