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

#[derive(serde::Deserialize)]
struct Body {
    id: String,
}

#[air_tight::permission(Workspace(Read), resources = [id])]
async fn handler(axum::Json(Body { id }): axum::Json<Body>) -> Result<String, air_tight::Error> {
    Ok(id)
}

fn main() {}
