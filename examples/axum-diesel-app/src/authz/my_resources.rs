use air_tight::ResourceKind;

/// Resource kinds that grants can be scoped to.
#[derive(ResourceKind)]
pub enum MyResources {
    /// A workspace.
    Workspace,
    /// An invoice.
    Invoice,
}
