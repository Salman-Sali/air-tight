//! Diesel table definitions for the `air_tight` Postgres schema.
//!
//! `user_roles` and `user_permissions` have no primary key in the database.
//! Diesel requires one, so all four columns of their `UNIQUE NULLS NOT DISTINCT`
//! constraint are declared as the key; this is only used for typing and never
//! for `find`.

diesel::table! {
    air_tight.permissions (key) {
        key -> Text,
        category -> Text,
        action -> Text,
        resource_kind -> Nullable<Text>,
        description -> Nullable<Text>,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    air_tight.resource_kinds (key) {
        key -> Text,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    air_tight.roles (id) {
        id -> Uuid,
        name -> Text,
        description -> Nullable<Text>,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    air_tight.role_permissions (role_id, permission_key) {
        role_id -> Uuid,
        permission_key -> Text,
    }
}

diesel::table! {
    air_tight.user_roles (user_id, role_id, scope_kind, scope_id) {
        user_id -> Text,
        role_id -> Uuid,
        scope_kind -> Nullable<Text>,
        scope_id -> Nullable<Text>,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    air_tight.user_permissions (user_id, permission_key, scope_kind, scope_id) {
        user_id -> Text,
        permission_key -> Text,
        scope_kind -> Nullable<Text>,
        scope_id -> Nullable<Text>,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    air_tight.audit_log (id) {
        id -> Int8,
        actor_user_id -> Nullable<Text>,
        action -> Text,
        target_user_id -> Nullable<Text>,
        role_id -> Nullable<Uuid>,
        permission_key -> Nullable<Text>,
        scope_kind -> Nullable<Text>,
        scope_id -> Nullable<Text>,
        detail -> Nullable<Jsonb>,
        at -> Timestamptz,
    }
}

diesel::joinable!(role_permissions -> roles (role_id));
diesel::joinable!(user_roles -> roles (role_id));

diesel::allow_tables_to_appear_in_same_query!(
    audit_log,
    permissions,
    resource_kinds,
    role_permissions,
    roles,
    user_permissions,
    user_roles,
);
