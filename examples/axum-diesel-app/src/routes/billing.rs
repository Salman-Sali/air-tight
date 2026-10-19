use axum::Json;

use crate::services::{BillingSummary, billing_summary};

/// `GET /billing`: needs the global `Billing(Read)`.
///
/// Uses the `granted` proof the macro provides to report whether the caller
/// can also manage billing, via an in-memory re-check.
#[air_tight::permission(Billing(Read))]
pub async fn billing() -> Json<BillingSummary> {
    Json(billing_summary(&granted))
}
