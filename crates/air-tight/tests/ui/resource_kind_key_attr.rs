use air_tight::ResourceKind;

#[derive(ResourceKind)]
enum Kinds {
    #[air_tight(key = "Ws")]
    Workspace,
}

fn main() {}
