use air_tight::{Permission, PermissionCategory};

#[derive(Permission)]
enum Action {
    Read,
}

struct NotAMarker;

#[derive(PermissionCategory)]
enum Perms {
    #[air_tight(resource = NotAMarker)]
    Workspace(Action),
}

fn main() {}
