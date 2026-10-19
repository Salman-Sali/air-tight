use air_tight::{Permission, PermissionCategory};

#[derive(Permission)]
enum Action {
    Read,
}

#[derive(PermissionCategory)]
enum Perms {
    Workspace(Action),
    #[air_tight(key = "Workspace")]
    Other(Action),
}

fn main() {}
