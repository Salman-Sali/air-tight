use std::fmt;

use diesel::prelude::*;
use diesel::sql_query;
use diesel::sql_types::Text;

use crate::{Error, NotifyChannel, RoleId};

/// One cache invalidation, as carried in a NOTIFY payload.
///
/// Payload forms are `user:<id>`, `role:<uuid>` and `all`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Invalidation {
    /// Drop one user's grant set.
    User(String),
    /// Drop the grant sets of every user holding a role.
    Role(RoleId),
    /// Drop every grant set.
    All,
}

impl Invalidation {
    /// Parses a NOTIFY payload, or `None` when it is not recognised.
    pub(crate) fn parse(payload: &str) -> Option<Self> {
        if payload == "all" {
            return Some(Invalidation::All);
        }
        if let Some(id) = payload.strip_prefix("user:") {
            return Some(Invalidation::User(id.to_owned()));
        }
        let uuid = payload.strip_prefix("role:")?;
        uuid::Uuid::parse_str(uuid)
            .ok()
            .map(|u| Invalidation::Role(RoleId::from_uuid(u)))
    }

    /// Sends this invalidation with `pg_notify`; delivered when the surrounding
    /// transaction commits.
    pub(crate) fn notify(
        &self,
        conn: &mut PgConnection,
        channel: &NotifyChannel,
    ) -> Result<(), Error> {
        sql_query("SELECT pg_notify($1, $2)")
            .bind::<Text, _>(channel.as_str())
            .bind::<Text, _>(self.to_string())
            .execute(conn)?;
        Ok(())
    }
}

impl fmt::Display for Invalidation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Invalidation::User(id) => write!(f, "user:{id}"),
            Invalidation::Role(id) => write!(f, "role:{id}"),
            Invalidation::All => f.write_str("all"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn payload_round_trip() {
        let role = RoleId::new();
        for inv in [
            Invalidation::All,
            Invalidation::User("u:1".into()),
            Invalidation::Role(role),
        ] {
            assert_eq!(Invalidation::parse(&inv.to_string()), Some(inv));
        }
        assert_eq!(Invalidation::parse("role:nope"), None);
        assert_eq!(Invalidation::parse("whatever"), None);
    }
}
