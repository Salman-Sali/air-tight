use air_tight::{IdentifyResources, ResourceKind};

#[derive(ResourceKind)]
enum Kinds {
    Workspace,
}

use kinds_kinds::Workspace;

#[derive(IdentifyResources)]
struct Body {
    #[air_tight(resource = Workspace, nested)]
    id: u8,
}

fn main() {}
