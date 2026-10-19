CREATE SCHEMA IF NOT EXISTS air_tight;

CREATE TABLE air_tight.permissions (
  key TEXT PRIMARY KEY,            -- "Workspace::Update"
  category TEXT NOT NULL,
  action TEXT NOT NULL,
  resource_kind TEXT NULL,
  description TEXT NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE air_tight.resource_kinds (
  key TEXT PRIMARY KEY,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE air_tight.roles (
  id UUID PRIMARY KEY,
  name TEXT NOT NULL UNIQUE,
  description TEXT NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE air_tight.role_permissions (
  role_id UUID NOT NULL REFERENCES air_tight.roles(id) ON DELETE CASCADE,
  permission_key TEXT NOT NULL,     -- may be a wildcard, so no FK
  PRIMARY KEY (role_id, permission_key)
);

CREATE TABLE air_tight.user_roles (
  user_id TEXT NOT NULL,
  role_id UUID NOT NULL REFERENCES air_tight.roles(id) ON DELETE CASCADE,
  scope_kind TEXT NULL,
  scope_id TEXT NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  UNIQUE NULLS NOT DISTINCT (user_id, role_id, scope_kind, scope_id),
  CHECK ((scope_kind IS NULL) = (scope_id IS NULL))
);

CREATE TABLE air_tight.user_permissions (
  user_id TEXT NOT NULL,
  permission_key TEXT NOT NULL,
  scope_kind TEXT NULL,
  scope_id TEXT NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  UNIQUE NULLS NOT DISTINCT (user_id, permission_key, scope_kind, scope_id),
  CHECK ((scope_kind IS NULL) = (scope_id IS NULL))
);

CREATE TABLE air_tight.audit_log (
  id BIGSERIAL PRIMARY KEY,
  actor_user_id TEXT NULL,
  action TEXT NOT NULL,            -- role_created, role_deleted, role_permissions_set, role_assigned, role_revoked, grant_added, grant_revoked, catalog_synced
  target_user_id TEXT NULL,
  role_id UUID NULL,
  permission_key TEXT NULL,
  scope_kind TEXT NULL,
  scope_id TEXT NULL,
  detail JSONB NULL,
  at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX ON air_tight.user_roles (user_id);
CREATE INDEX ON air_tight.user_roles (role_id);
CREATE INDEX ON air_tight.user_roles (scope_kind, scope_id);
CREATE INDEX ON air_tight.user_permissions (user_id);
CREATE INDEX ON air_tight.user_permissions (scope_kind, scope_id);
