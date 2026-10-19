use diesel::PgConnection;
use diesel_migrations::{EmbeddedMigrations, MigrationHarness, embed_migrations};

/// The example's own migrations (`users`, `workspaces`, `invoices`), separate
/// from the `air_tight` schema that air-tight migrates itself.
const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations");

/// Applies pending migrations of the example's tables.
pub fn migrate(conn: &mut PgConnection) -> Result<(), String> {
    conn.run_pending_migrations(MIGRATIONS)
        .map(|_| ())
        .map_err(|err| err.to_string())
}
