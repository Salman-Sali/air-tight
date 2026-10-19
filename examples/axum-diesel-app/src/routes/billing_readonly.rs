/// `GET /billing/readonly`: for billing readers who are not managers.
///
/// Demonstrates `!`, which means "lacks the permission" (it is never a deny
/// rule): needs `Billing(Read)` and must not hold `Billing(Manage)`.
#[air_tight::permission(Billing(Read) && !Billing(Manage))]
pub async fn billing_readonly() -> &'static str {
    "read-only billing view"
}
