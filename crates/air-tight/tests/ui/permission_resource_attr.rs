use air_tight::Permission;

#[derive(Permission)]
enum Action {
    #[air_tight(resource = Workspace)]
    Read,
}

fn main() {}
