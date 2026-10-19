//! The curl examples printed at startup.
use crate::seed::{ALICE, BOB, CAROL, INVOICE_A, INVOICE_B, WORKSPACE_A, WORKSPACE_B};

/// The curl commands printed at startup, for a server at `base`.
pub fn curl_examples(base: &str) -> String {
    let get = |user: Option<&str>, path: &str, note: &str| {
        let header = user.map_or(String::new(), |u| format!(" -H 'X-User-Id: {u}'"));
        format!("  curl -i{header} {base}{path}    # {note}\n")
    };
    let mut out = String::from(
        "Try these (seeded users: alice = everything, bob = Member of workspace A, carol = nothing):\n",
    );
    out += &get(None, "/health", "200, no auth needed");
    out += &get(None, "/workspaces", "401, no user");
    out += &get(
        Some(&ALICE.to_string()),
        "/workspaces",
        "200, both workspaces",
    );
    out += &get(
        Some(&BOB.to_string()),
        "/workspaces",
        "200, workspace A only",
    );
    out += &get(Some(&CAROL.to_string()), "/workspaces", "200, empty list");
    out += &get(
        Some(&BOB.to_string()),
        &format!("/workspaces/{WORKSPACE_A}"),
        "200",
    );
    out += &get(
        Some(&BOB.to_string()),
        &format!("/workspaces/{WORKSPACE_B}"),
        "403",
    );
    out += &get(
        Some(&ALICE.to_string()),
        &format!("/invoices/{INVOICE_A}"),
        "200",
    );
    out += &get(
        Some(&CAROL.to_string()),
        &format!("/invoices/{INVOICE_B}"),
        "403",
    );
    out += &get(Some(&ALICE.to_string()), "/billing", "200");
    out += &get(Some(&BOB.to_string()), "/billing", "403");
    out += "See examples/axum-diesel-app/README.md and curl.sh for the full list.\n";
    out
}
