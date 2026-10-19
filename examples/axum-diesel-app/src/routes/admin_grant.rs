use air_tight::{CurrentUser, PermissionKey};
use axum::Json;
use axum::extract::State;
use serde_json::{Value, json};

use crate::auth::AuthenticatedUser;
use crate::db::blocking;
use crate::error::ApiError;
use crate::requests::GrantPermission;
use crate::state::Client;

/// `POST /admin/grant`: grants a permission key directly to a user, optionally
/// within one resource. Needs `Billing(Manage)`.
#[air_tight::permission(Billing(Manage), state = client, user = me)]
pub async fn admin_grant(
    State(client): State<Client>,
    CurrentUser(me): CurrentUser<AuthenticatedUser>,
    Json(body): Json<GrantPermission>,
) -> Result<Json<Value>, ApiError> {
    let actor = me.id.to_string();
    blocking(move || {
        let scope = body.scope.map(|s| s.into_scope()).transpose()?;
        client.grant(
            Some(&actor),
            &body.user_id,
            PermissionKey::new(body.permission),
            scope,
        )?;
        Ok(())
    })
    .await?;
    Ok(Json(json!({ "granted": true })))
}
