use air_tight::{Permission, PermissionCategory, Principal, ResourceKind};

#[derive(ResourceKind)]
enum Kinds {
    Workspace,
}

#[derive(Permission)]
enum WsAction {
    Read,
}

#[derive(PermissionCategory)]
enum Perms {
    #[air_tight(resource = kinds_kinds::Workspace)]
    Workspace(WsAction),
}

#[derive(Clone)]
struct User;

impl Principal for User {
    fn air_tight_id(&self) -> String {
        String::new()
    }
}

air_tight::setup!(perms = Perms, user = User, resources = Kinds);

#[air_tight::permission(Workspace(Read) Workspace(Read))]
async fn handler() -> &'static str {
    "two leaves"
}

fn main() {}
