use diesel::prelude::*;
use uuid::Uuid;

use crate::schema::invoices;

/// A row to insert into `invoices`.
#[derive(Clone, Debug, Insertable)]
#[diesel(table_name = invoices)]
pub struct NewInvoice {
    /// The invoice id.
    pub id: Uuid,
    /// The workspace the invoice belongs to.
    pub workspace_id: Uuid,
    /// The amount in cents.
    pub amount: i64,
}
