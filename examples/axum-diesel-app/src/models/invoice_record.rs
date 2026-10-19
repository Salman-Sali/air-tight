use diesel::prelude::*;
use serde::Serialize;
use uuid::Uuid;

use crate::schema::invoices;

/// A row of `invoices`.
#[derive(Clone, Debug, Queryable, Selectable, Serialize)]
#[diesel(table_name = invoices)]
pub struct InvoiceRecord {
    /// The invoice id.
    pub id: Uuid,
    /// The workspace the invoice belongs to.
    pub workspace_id: Uuid,
    /// The amount in cents.
    pub amount: i64,
}
