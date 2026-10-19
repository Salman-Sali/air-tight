//! Diesel table definitions of the example's own tables.

diesel::table! {
    users (id) {
        id -> Uuid,
        name -> Text,
    }
}

diesel::table! {
    workspaces (id) {
        id -> Uuid,
        name -> Text,
    }
}

diesel::table! {
    invoices (id) {
        id -> Uuid,
        workspace_id -> Uuid,
        amount -> BigInt,
    }
}

diesel::joinable!(invoices -> workspaces (workspace_id));
diesel::allow_tables_to_appear_in_same_query!(users, workspaces, invoices);
