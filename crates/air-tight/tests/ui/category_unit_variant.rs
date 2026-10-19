use air_tight::{Permission, PermissionCategory};

#[derive(Permission)]
enum Action {
    Read,
}

#[derive(PermissionCategory)]
enum Perms {
    Workspace(Action),
    Billing,
}

fn main() {}
