use air_tight::ResourceKind;

#[derive(ResourceKind)]
enum Kinds {
    Workspace,
    Invoice { id: u8 },
}

fn main() {}
