use air_tight::{CurrentUser, RoleId};
use axum::Json;
use axum::extract::State;
use serde_json::{Value, json};

use crate::auth::AuthenticatedUser;
use crate::db::blocking;
use crate::error::ApiError;
use crate::requests::AssignRole;
use crate::state::Client;

/// `POST /admin/assign`: assigns a role to a user, optionally within one
/// resource. Needs `Billing(Manage)`.
#[air_tight::permission(Billing(Manage), state = client, user = me)]
pub async fn admin_assign(
    State(client): State<Client>,
    CurrentUser(me): CurrentUser<AuthenticatedUser>,
    Json(body): Json<AssignRole>,
) -> Result<Json<Value>, ApiError> {
    let actor = me.id.to_string();
    blocking(move || {
        let scope = body.scope.map(|s| s.into_scope()).transpose()?;
        client.assign_role(
            Some(&actor),
            &body.user_id,
            RoleId::from_uuid(body.role_id),
            scope,
        )?;
        Ok(())
    })
    .await?;
    Ok(Json(json!({ "assigned": true })))
}
