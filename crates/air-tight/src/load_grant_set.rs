use diesel::prelude::*;

use crate::schema::{role_permissions, user_permissions, user_roles};
use crate::{Error, GrantSet, Interner};

type Row = (String, Option<String>, Option<String>);

/// Loads the flattened grants of `user_id` using exactly two queries.
///
/// The first reads direct grants from `user_permissions`; the second joins
/// `user_roles` to `role_permissions`, applying each role permission within the
/// role assignment's scope. Keys and kinds are interned lazily.
pub fn load_grant_set(conn: &mut PgConnection, user_id: &str) -> Result<GrantSet, Error> {
    let direct: Vec<Row> = user_permissions::table
        .filter(user_permissions::user_id.eq(user_id))
        .select((
            user_permissions::permission_key,
            user_permissions::scope_kind,
            user_permissions::scope_id,
        ))
        .load(conn)?;
    let via_roles: Vec<Row> = user_roles::table
        .inner_join(role_permissions::table.on(role_permissions::role_id.eq(user_roles::role_id)))
        .filter(user_roles::user_id.eq(user_id))
        .select((
            role_permissions::permission_key,
            user_roles::scope_kind,
            user_roles::scope_id,
        ))
        .load(conn)?;

    let interner = Interner::global();
    let mut set = GrantSet::new();
    for (key, kind, id) in direct.into_iter().chain(via_roles) {
        let key = interner.intern_key(&key)?;
        match (kind, id) {
            (Some(kind), Some(id)) => set.insert_scoped(key, interner.intern_kind(&kind)?, &id),
            _ => set.insert_global(key),
        }
    }
    Ok(set)
}
