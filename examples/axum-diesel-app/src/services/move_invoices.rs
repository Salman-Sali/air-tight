use diesel::prelude::*;
use uuid::Uuid;

use crate::db::Db;
use crate::error::ApiError;
use crate::requests::MoveInvoice;
use crate::schema::invoices;

/// Moves the invoices named by `request` into its target workspace and returns
/// how many moved.
///
/// Plain function: the permission check happens in the handler that calls it.
pub async fn move_invoices(db: &Db, request: &MoveInvoice) -> Result<usize, ApiError> {
    let mut ids: Vec<Uuid> = request.related_invoice_ids.clone();
    ids.push(request.invoice_id);
    let target = request.target_workspace_id;
    let moved = db
        .run(move |conn| {
            Ok(
                diesel::update(invoices::table.filter(invoices::id.eq_any(ids)))
                    .set(invoices::workspace_id.eq(target))
                    .execute(conn)?,
            )
        })
        .await?;
    if moved == 0 {
        return Err(ApiError::NotFound("no such invoice"));
    }
    Ok(moved)
}
