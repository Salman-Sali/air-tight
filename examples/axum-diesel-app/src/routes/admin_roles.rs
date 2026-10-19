use air_tight::{CurrentUser, PermissionKey, RoleName};
use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use serde_json::{Value, json};

use crate::auth::AuthenticatedUser;
use crate::db::blocking;
use crate::error::ApiError;
use crate::requests::CreateRole;
use crate::state::Client;

/// `POST /admin/roles`: creates a role with a set of permission keys. Needs
/// `Billing(Manage)`, which stands in for an admin permission.
#[air_tight::permission(Billing(Manage), state = client, user = me)]
pub async fn admin_roles(
    State(client): State<Client>,
    CurrentUser(me): CurrentUser<AuthenticatedUser>,
    Json(body): Json<CreateRole>,
) -> Result<(StatusCode, Json<Value>), ApiError> {
    let actor = me.id.to_string();
    let id = blocking(move || {
        let name = RoleName::new(&body.name)?;
        let keys: Vec<PermissionKey> = body
            .permissions
            .into_iter()
            .map(PermissionKey::new)
            .collect();
        let id = client.create_role(Some(&actor), &name, body.description.as_deref())?;
        client.set_role_permissions(Some(&actor), id, &keys)?;
        Ok(id)
    })
    .await?;
    Ok((StatusCode::CREATED, Json(json!({ "id": id }))))
}
