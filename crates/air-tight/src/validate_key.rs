use air_tight_core::{PermissionKey, is_well_formed_wildcard};
use diesel::prelude::*;

use crate::Error;
use crate::schema::permissions;

/// Fails with `Error::Catalog` unless every key is in the catalog or is a
/// well-formed wildcard (`*` or `<Category>::*`).
///
/// The error names the first offending key.
pub(crate) fn ensure_known_keys<'a>(
    conn: &mut PgConnection,
    keys: impl IntoIterator<Item = &'a PermissionKey>,
) -> Result<(), Error> {
    let concrete: Vec<&str> = keys
        .into_iter()
        .map(PermissionKey::as_str)
        .filter(|k| !is_well_formed_wildcard(k))
        .collect();
    if concrete.is_empty() {
        return Ok(());
    }
    let known: Vec<String> = permissions::table
        .filter(permissions::key.eq_any(&concrete))
        .select(permissions::key)
        .load(conn)?;
    match concrete.iter().find(|k| !known.iter().any(|s| s == **k)) {
        Some(bad) => Err(Error::Catalog(format!(
            "unknown permission key `{bad}`: not in the catalog and not a wildcard"
        ))),
        None => Ok(()),
    }
}
