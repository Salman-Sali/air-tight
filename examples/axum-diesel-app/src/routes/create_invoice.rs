use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use diesel::prelude::*;
use uuid::Uuid;

use crate::db::Db;
use crate::error::ApiError;
use crate::models::{InvoiceRecord, NewInvoice};
use crate::requests::CreateInvoice;
use crate::schema::invoices;

/// `POST /invoices`: needs `Workspace(Update)` on the body's workspace and the
/// global `Invoice(Create)`.
#[air_tight::permission(Workspace(Update) && Invoice(Create), resources = [body])]
pub async fn create_invoice(
    State(db): State<Db>,
    Json(body): Json<CreateInvoice>,
) -> Result<(StatusCode, Json<InvoiceRecord>), ApiError> {
    let row = db
        .run(move |conn| {
            Ok(diesel::insert_into(invoices::table)
                .values(NewInvoice {
                    id: Uuid::new_v4(),
                    workspace_id: body.workspace_id,
                    amount: body.amount,
                })
                .returning(InvoiceRecord::as_returning())
                .get_result(conn)?)
        })
        .await?;
    Ok((StatusCode::CREATED, Json(row)))
}
