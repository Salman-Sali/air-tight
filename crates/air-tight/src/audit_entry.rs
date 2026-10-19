use diesel::prelude::*;
use serde_json::Value;

use crate::audit_action::AuditAction;
use crate::schema::audit_log;
use crate::{Error, ResourceScope, RoleId};

/// A row of `air_tight.audit_log` being assembled by a management call.
#[derive(Insertable)]
#[diesel(table_name = audit_log)]
pub(crate) struct AuditEntry<'a> {
    actor_user_id: Option<&'a str>,
    action: &'static str,
    target_user_id: Option<&'a str>,
    role_id: Option<uuid::Uuid>,
    permission_key: Option<&'a str>,
    scope_kind: Option<&'a str>,
    scope_id: Option<&'a str>,
    detail: Option<Value>,
}

impl<'a> AuditEntry<'a> {
    /// Starts an entry for `action` performed by `actor`.
    pub(crate) fn new(actor: Option<&'a str>, action: AuditAction) -> Self {
        AuditEntry {
            actor_user_id: actor,
            action: action.as_str(),
            target_user_id: None,
            role_id: None,
            permission_key: None,
            scope_kind: None,
            scope_id: None,
            detail: None,
        }
    }

    pub(crate) fn target_user(mut self, user_id: &'a str) -> Self {
        self.target_user_id = Some(user_id);
        self
    }

    pub(crate) fn role(mut self, role: RoleId) -> Self {
        self.role_id = Some(role.as_uuid());
        self
    }

    pub(crate) fn permission(mut self, key: &'a str) -> Self {
        self.permission_key = Some(key);
        self
    }

    pub(crate) fn scope(mut self, scope: Option<&'a ResourceScope>) -> Self {
        self.scope_kind = scope.map(|s| s.kind.as_str());
        self.scope_id = scope.map(|s| s.id.as_str());
        self
    }

    pub(crate) fn detail(mut self, detail: Value) -> Self {
        self.detail = Some(detail);
        self
    }

    /// Inserts the row on `conn`.
    pub(crate) fn write(self, conn: &mut PgConnection) -> Result<(), Error> {
        diesel::insert_into(audit_log::table)
            .values(&self)
            .execute(conn)?;
        Ok(())
    }
}
