//! Behaviour tests for the derive macros.

use std::collections::HashSet;

use air_tight::{
    IdentifyResources, Permission, PermissionCategory, PermissionKey, ResourceKind,
    ResourceKindKey, ResourceKindMarker, ResourceScope,
};
use uuid::Uuid;

#[derive(ResourceKind)]
pub enum MyResources {
    Workspace,
    Invoice,
}

use my_resources_kinds::{Invoice, Workspace};

#[derive(Permission)]
pub enum WorkspaceAction {
    Read,
    Update,
    #[air_tight(key = "Remove")]
    Delete,
}

#[derive(Permission)]
pub enum BillingAction {
    Read,
    Manage,
}

#[derive(PermissionCategory)]
pub enum MyPerms {
    #[air_tight(resource = Workspace)]
    Workspace(WorkspaceAction),
    #[air_tight(key = "Money")]
    Billing(BillingAction),
}

#[test]
fn permission_names_and_all() {
    assert_eq!(WorkspaceAction::Read.action_name(), "Read");
    assert_eq!(WorkspaceAction::Delete.action_name(), "Remove");
    assert_eq!(<WorkspaceAction as Permission>::all().len(), 3);
    assert_eq!(format!("{:?}", WorkspaceAction::Update), "Update");
}

#[test]
fn category_keys_and_all() {
    let keys: Vec<String> = <MyPerms as PermissionCategory>::all()
        .iter()
        .map(|p| p.key().to_string())
        .collect();
    assert_eq!(
        keys,
        [
            "Workspace::Read",
            "Workspace::Update",
            "Workspace::Remove",
            "Money::Read",
            "Money::Manage"
        ]
    );
    assert_eq!(
        MyPerms::Workspace(WorkspaceAction::Update).key(),
        PermissionKey::from_static("Workspace::Update")
    );
    assert_eq!(
        MyPerms::Billing(BillingAction::Read).category_name(),
        "Money"
    );
    assert_eq!(
        format!("{:?}", MyPerms::Billing(BillingAction::Manage)),
        "Billing(Manage)"
    );
}

#[test]
fn category_from_key_round_trips() {
    for p in <MyPerms as PermissionCategory>::all() {
        assert_eq!(MyPerms::from_key(p.key().as_str()), Some(*p));
    }
    assert_eq!(MyPerms::from_key("Workspace::Delete"), None);
    assert_eq!(MyPerms::from_key("Billing::Read"), None);
    assert_eq!(MyPerms::from_key("Workspace::*"), None);
    assert_eq!(MyPerms::from_key("nope"), None);
}

#[test]
fn category_resource_kind() {
    assert_eq!(
        MyPerms::Workspace(WorkspaceAction::Read).resource_kind(),
        Some(ResourceKindKey::from_static("Workspace"))
    );
    assert_eq!(MyPerms::Billing(BillingAction::Read).resource_kind(), None);
}

#[test]
fn equality_and_hash() {
    let set: HashSet<MyPerms> = <MyPerms as PermissionCategory>::all()
        .iter()
        .copied()
        .collect();
    assert_eq!(set.len(), 5);
    assert_ne!(
        MyPerms::Workspace(WorkspaceAction::Read),
        MyPerms::Workspace(WorkspaceAction::Update)
    );
}

#[test]
fn resource_kind_derive() {
    assert_eq!(MyResources::Invoice.key().as_str(), "Invoice");
    assert_eq!(<MyResources as ResourceKind>::all().len(), 2);
    assert_eq!(
        MyResources::from_key("Workspace"),
        Some(MyResources::Workspace)
    );
    assert_eq!(MyResources::from_key("Project"), None);
    assert_eq!(<Invoice as ResourceKindMarker>::key().as_str(), "Invoice");
    assert_eq!(MyResources::from(Workspace), MyResources::Workspace);
}

#[derive(IdentifyResources)]
pub struct LineItem {
    #[air_tight(resource = Invoice)]
    pub invoice: u32,
}

#[derive(IdentifyResources)]
pub struct BillingSection {
    #[air_tight(resource = Workspace)]
    pub owner: String,
}

#[derive(IdentifyResources)]
pub struct MoveInvoiceRequest {
    #[air_tight(resource = Invoice)]
    pub invoice_id: Uuid,
    #[air_tight(resource = Workspace)]
    pub target_workspace_id: Uuid,
    #[air_tight(resource = Invoice)]
    pub related: Vec<Uuid>,
    #[air_tight(resource = Workspace)]
    pub source: Option<Uuid>,
    #[air_tight(nested)]
    pub billing: BillingSection,
    #[air_tight(nested)]
    pub lines: Vec<LineItem>,
    #[allow(dead_code)]
    pub note: String,
}

fn scope(kind: &'static str, id: impl Into<String>) -> ResourceScope {
    ResourceScope::new(ResourceKindKey::from_static(kind), id)
}

#[test]
fn identify_struct() {
    let (a, b, c) = (Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4());
    let req = MoveInvoiceRequest {
        invoice_id: a,
        target_workspace_id: b,
        related: vec![c],
        source: None,
        billing: BillingSection { owner: "w9".into() },
        lines: vec![LineItem { invoice: 1 }, LineItem { invoice: 2 }],
        note: "ignored".into(),
    };
    assert_eq!(
        req.resource_scopes(),
        [
            scope("Invoice", a.to_string()),
            scope("Workspace", b.to_string()),
            scope("Invoice", c.to_string()),
            scope("Workspace", "w9"),
            scope("Invoice", "1"),
            scope("Invoice", "2"),
        ]
    );
}

#[derive(IdentifyResources)]
pub struct Tuple(#[air_tight(resource = Workspace)] pub u64, pub String);

#[derive(IdentifyResources)]
pub struct Unit;

#[derive(IdentifyResources)]
pub enum Command {
    Rename {
        #[air_tight(resource = Workspace)]
        workspace: u64,
        #[allow(dead_code)]
        name: String,
    },
    Pay(
        #[air_tight(resource = Invoice)] Vec<u64>,
        #[air_tight(nested)] Option<Tuple>,
    ),
    Noop,
}

#[test]
fn identify_tuple_unit_enum() {
    assert_eq!(
        Tuple(4, "x".into()).resource_scopes(),
        [scope("Workspace", "4")]
    );
    assert!(Unit.resource_scopes().is_empty());
    let rename = Command::Rename {
        workspace: 7,
        name: "n".into(),
    };
    assert_eq!(rename.resource_scopes(), [scope("Workspace", "7")]);
    let pay = Command::Pay(vec![1, 2], Some(Tuple(3, String::new())));
    assert_eq!(
        pay.resource_scopes(),
        [
            scope("Invoice", "1"),
            scope("Invoice", "2"),
            scope("Workspace", "3")
        ]
    );
    assert!(Command::Noop.resource_scopes().is_empty());
}

#[derive(IdentifyResources)]
pub struct Generic<T> {
    #[air_tight(resource = Workspace)]
    pub id: T,
}

#[test]
fn identify_generic() {
    assert_eq!(
        Generic { id: 5u8 }.resource_scopes(),
        [scope("Workspace", "5")]
    );
}

#[test]
fn ui() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/ui/*.rs");
}
