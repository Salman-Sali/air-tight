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

async fn plain() -> &'static str {
    "not guarded"
}

fn main() {
    let _ = air_tight::Router::<()>::new().route("/", air_tight::routing::get(plain));
}
