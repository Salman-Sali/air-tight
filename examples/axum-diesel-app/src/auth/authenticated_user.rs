use air_tight::Principal;

use super::UserId;
use crate::models::UserRecord;

/// The principal: the user a request was authenticated as.
///
/// The auth middleware puts it into the request extensions, where
/// `#[air_tight::permission]` finds it.
#[derive(Clone, Debug)]
pub struct AuthenticatedUser {
    /// The user's id.
    pub id: UserId,
    /// The user's display name.
    pub name: String,
}

impl From<UserRecord> for AuthenticatedUser {
    fn from(row: UserRecord) -> Self {
        AuthenticatedUser {
            id: UserId::from_uuid(row.id),
            name: row.name,
        }
    }
}

impl Principal for AuthenticatedUser {
    fn air_tight_id(&self) -> String {
        self.id.to_string()
    }
}
