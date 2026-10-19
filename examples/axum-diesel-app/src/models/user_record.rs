use diesel::prelude::*;
use uuid::Uuid;

use crate::schema::users;

/// A row of `users`.
#[derive(Clone, Debug, Queryable, Selectable, Insertable)]
#[diesel(table_name = users)]
pub struct UserRecord {
    /// The user id.
    pub id: Uuid,
    /// The display name.
    pub name: String,
}
