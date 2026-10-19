use diesel::connection::SimpleConnection;
use diesel::prelude::*;
use diesel::sql_query;
use diesel::sql_types::Text;

use crate::Error;

/// Fixed key of the advisory lock serialising migration runs.
const MIGRATION_LOCK_KEY: i64 = 0x41_49_52_54_49_47_48_54;

const MIGRATIONS: &[(&str, &str)] = &[("0001_init", include_str!("../migrations/0001_init.sql"))];

/// The embedded migrations as `(version, sql)` pairs in application order.
///
/// Exposed for people who manage migrations themselves. The `air_tight.migrations`
/// bookkeeping table is created by [`run_migrations`] and is not part of these files.
pub fn migration_sql() -> &'static [(&'static str, &'static str)] {
    MIGRATIONS
}

#[derive(QueryableByName)]
struct Applied {
    #[diesel(sql_type = diesel::sql_types::Bool)]
    applied: bool,
}

fn lock_and_bootstrap(conn: &mut PgConnection) -> QueryResult<()> {
    sql_query("SELECT pg_advisory_xact_lock($1)")
        .bind::<diesel::sql_types::BigInt, _>(MIGRATION_LOCK_KEY)
        .execute(conn)?;
    conn.batch_execute(
        "CREATE SCHEMA IF NOT EXISTS air_tight;
         CREATE TABLE IF NOT EXISTS air_tight.migrations (
           version TEXT PRIMARY KEY,
           applied_at TIMESTAMPTZ NOT NULL DEFAULT now()
         );",
    )
}

/// Applies every unapplied embedded migration, recording it in `air_tight.migrations`.
///
/// Each version runs in its own transaction guarded by an advisory lock, so
/// concurrent callers are safe. Running it again is a no-op. It never touches
/// diesel's or sqlx's own migration tables.
pub fn run_migrations(conn: &mut PgConnection) -> Result<(), Error> {
    conn.transaction(lock_and_bootstrap)?;
    for (version, sql) in migration_sql() {
        conn.transaction::<(), diesel::result::Error, _>(|conn| {
            lock_and_bootstrap(conn)?;
            let row: Applied = sql_query(
                "SELECT EXISTS (SELECT 1 FROM air_tight.migrations WHERE version = $1) AS applied",
            )
            .bind::<Text, _>(*version)
            .get_result(conn)?;
            if row.applied {
                return Ok(());
            }
            conn.batch_execute(sql)?;
            sql_query("INSERT INTO air_tight.migrations (version) VALUES ($1)")
                .bind::<Text, _>(*version)
                .execute(conn)?;
            Ok(())
        })?;
    }
    Ok(())
}
