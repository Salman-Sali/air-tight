CREATE TABLE users (
    id uuid PRIMARY KEY,
    name text NOT NULL
);

CREATE TABLE workspaces (
    id uuid PRIMARY KEY,
    name text NOT NULL
);

CREATE TABLE invoices (
    id uuid PRIMARY KEY,
    workspace_id uuid NOT NULL REFERENCES workspaces (id),
    amount bigint NOT NULL
);

CREATE INDEX invoices_workspace_id_idx ON invoices (workspace_id);
