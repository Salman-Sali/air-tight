use air_tight::AirTightPath;
use axum::Json;
use axum::extract::State;
use diesel::prelude::*;
use uuid::Uuid;

use crate::authz::my_resources_kinds::Invoice;
use crate::db::Db;
use crate::error::ApiError;
use crate::models::InvoiceRecord;
use crate::schema::invoices;

/// `GET /invoices/{id}`: needs `Invoice(Read)` on that invoice, or the global
/// `Workspace(Update)` (a workspace editor may read any invoice).
#[air_tight::permission(Invoice(Read) || Workspace(Update))]
pub async fn get_invoice(
    State(db): State<Db>,
    id: AirTightPath<Uuid, Invoice>,
) -> Result<Json<InvoiceRecord>, ApiError> {
    let id = id.into_inner();
    let row = db
        .run(move |conn| {
            Ok(invoices::table
                .find(id)
                .select(InvoiceRecord::as_select())
                .first(conn)?)
        })
        .await?;
    Ok(Json(row))
}
