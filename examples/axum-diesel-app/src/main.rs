//! Example application for air-tight (axum + diesel).

mod authz;

use air_tight::{PermissionCategory, ResourceKind};

use authz::{MyPerms, MyResources};

fn main() {
    // Placeholder until the server lands: print the catalog.
    for p in MyPerms::all() {
        println!("permission {}", p.key());
    }
    for k in MyResources::all() {
        println!("resource kind {}", k.key());
    }
}
