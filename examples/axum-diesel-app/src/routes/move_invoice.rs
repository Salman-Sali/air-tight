use axum::Json;
use axum::extract::State;
use serde_json::{Value, json};

use crate::db::Db;
use crate::error::ApiError;
use crate::requests::MoveInvoice;
use crate::services;

/// `POST /invoices/move`: needs `Invoice(Update)` on every invoice in the body
/// (the invoice and its related ones) and `Workspace(Update)` on the target
/// workspace.
#[air_tight::permission(all(Invoice(Update), Workspace(Update)), resources = [body])]
pub async fn move_invoice(
    State(db): State<Db>,
    Json(body): Json<MoveInvoice>,
) -> Result<Json<Value>, ApiError> {
    let moved = services::move_invoices(&db, &body).await?;
    Ok(Json(json!({ "moved": moved })))
}
