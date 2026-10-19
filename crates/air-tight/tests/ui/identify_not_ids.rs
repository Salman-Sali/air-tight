use air_tight::{IdentifyResources, ResourceKind};

#[derive(ResourceKind)]
enum Kinds {
    Workspace,
}

use kinds_kinds::Workspace;

struct Opaque;

#[derive(IdentifyResources)]
struct Body {
    #[air_tight(resource = Workspace)]
    id: Opaque,
}

fn main() {}
