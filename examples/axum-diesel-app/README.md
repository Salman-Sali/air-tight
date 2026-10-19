# axum-diesel-app

A runnable axum + diesel + Postgres app protected with air-tight. It is also
the integration test of the crate: `tests/` drives the same router.

## Run it

```sh
docker compose up -d                 # from the repository root, Postgres on port 55432
cargo run -p axum-diesel-app
```

The compose file creates two databases: `air_tight_test` (used by the test
suites) and `air_tight_example` (used by this app). The init script only runs
when the container is created, so after upgrading an existing container run
`docker compose down -v && docker compose up -d`.

| Variable | Default |
| --- | --- |
| `DATABASE_URL` | `postgres://postgres:password@localhost:55432/air_tight_example` |
| `BIND_ADDR` | `127.0.0.1:3000` |

On start the app applies its own migrations (`users`, `workspaces`,
`invoices`, separate from the `air_tight` schema that air-tight manages),
builds the air-tight client, seeds demo data (safe to repeat), audits the
routes with `audit_routes` and prints curl examples.

## Authentication

A fake middleware reads the `X-User-Id` header and loads that user from the
`users` table. A missing, malformed or unknown id answers `401`. Everything
except `/health` sits behind it.

## Seed data

| User | Id | Access |
| --- | --- | --- |
| alice | `aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa` | global `*` |
| bob | `bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb` | role `Member` (`Workspace::Read`, `Invoice::Read`) scoped to workspace A |
| carol | `cccccccc-cccc-cccc-cccc-cccccccccccc` | nothing |

Workspace A is `11111111-1111-1111-1111-111111111111`, workspace B is
`22222222-2222-2222-2222-222222222222`. Invoice `a1a1a1a1-a1a1-a1a1-a1a1-a1a1a1a1a1a1`
is in A and `b1b1b1b1-b1b1-b1b1-b1b1-b1b1b1b1b1b1` is in B.

Note that bob's `Invoice::Read` is scoped to the *workspace* A, while the
`Invoice` category checks grants on *invoice* scopes, so it does not match any
invoice request. Grant `Invoice::Read` scoped to an invoice (or globally) to
see it take effect.

## Routes

| Route | Requires | Notes |
| --- | --- | --- |
| `GET /health` | nothing | `unrestricted`, no authentication |
| `GET /workspaces` | any user | lists readable workspaces with `accessible_ids` (`All` / `Some` / `None`) |
| `GET /workspaces/sql` | any user | same list through the diesel `accessible` expression |
| `GET /workspaces/{id}` | `Workspace(Read)` | `AirTightPath` identifies the workspace |
| `PATCH /workspaces/{id}` | `Workspace(Update)` | delegates to a `kind = fn` service that checks the same permission |
| `POST /invoices` | `Workspace(Update) && Invoice(Create)` | body `{"workspace_id", "amount"}` |
| `POST /invoices/move` | `all(Invoice(Update), Workspace(Update))` | body `{"invoice_id", "related_invoice_ids": [..], "target_workspace_id"}`; every invoice id is checked |
| `GET /invoices/{id}` | `Invoice(Read) \|\| Workspace(Update)` | |
| `GET /billing` | `Billing(Read)` | reports `can_manage` using `Granted::require` |
| `GET /billing/readonly` | `Billing(Read) && !Billing(Manage)` | `!` means "lacks the permission" |
| `POST /admin/roles` | `Billing(Manage)` | body `{"name", "description"?, "permissions": [..]}` |
| `POST /admin/assign` | `Billing(Manage)` | body `{"user_id", "role_id", "scope"?: {"kind", "id"}}` |
| `POST /admin/grant` | `Billing(Manage)` | body `{"user_id", "permission", "scope"?: {"kind", "id"}}` |

Authorization is checked before the handler touches the database, so a caller
without the permission gets `403` whether or not the resource exists.

## curl

```sh
BASE=http://127.0.0.1:3000
ALICE=aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa
BOB=bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb
CAROL=cccccccc-cccc-cccc-cccc-cccccccccccc
A=11111111-1111-1111-1111-111111111111
B=22222222-2222-2222-2222-222222222222
```

| Command | Status |
| --- | --- |
| `curl -i $BASE/health` | 200 |
| `curl -i $BASE/workspaces` | 401 (no user) |
| `curl -i -H "X-User-Id: $ALICE" $BASE/workspaces` | 200, both workspaces |
| `curl -i -H "X-User-Id: $BOB" $BASE/workspaces` | 200, workspace A only |
| `curl -i -H "X-User-Id: $CAROL" $BASE/workspaces/sql` | 200, empty list |
| `curl -i -H "X-User-Id: $BOB" $BASE/workspaces/$A` | 200 |
| `curl -i -H "X-User-Id: $BOB" $BASE/workspaces/$B` | 403 |
| `curl -i -H "X-User-Id: $ALICE" $BASE/workspaces/$B` | 200 (wildcard) |
| `curl -i -X PATCH -H "X-User-Id: $BOB" -H 'Content-Type: application/json' -d '{"name":"x"}' $BASE/workspaces/$A` | 403 |
| `curl -i -X PATCH -H "X-User-Id: $ALICE" -H 'Content-Type: application/json' -d '{"name":"Workspace A"}' $BASE/workspaces/$A` | 200 |
| `curl -i -X POST -H "X-User-Id: $ALICE" -H 'Content-Type: application/json' -d "{\"workspace_id\":\"$A\",\"amount\":500}" $BASE/invoices` | 201 |
| `curl -i -X POST -H "X-User-Id: $BOB" -H 'Content-Type: application/json' -d "{\"workspace_id\":\"$A\",\"amount\":500}" $BASE/invoices` | 403 |
| `curl -i -H "X-User-Id: $ALICE" $BASE/billing` | 200, `{"can_manage":true}` |
| `curl -i -H "X-User-Id: $BOB" $BASE/billing` | 403 |
| `curl -i -H "X-User-Id: $ALICE" $BASE/billing/readonly` | 403 (alice can manage) |
| `curl -i -X POST -H "X-User-Id: $ALICE" -H 'Content-Type: application/json' -d '{"name":"Auditors","permissions":["Invoice::Read"]}' $BASE/admin/roles` | 201, `{"id":"..."}` |
| `curl -i -X POST -H "X-User-Id: $BOB" -H 'Content-Type: application/json' -d '{"user_id":"x","permission":"Billing::Read"}' $BASE/admin/grant` | 403 |

`./examples/axum-diesel-app/curl.sh` runs a longer list (about 40 requests)
against a running server and fails if any status differs from the expected one.

## Tests

```sh
docker compose up -d
AIR_TIGHT_TEST_DATABASE_URL=postgres://postgres:password@localhost:55432/air_tight_test \
  cargo test -p axum-diesel-app
```

Without the variable the database tests are skipped. They create unique
users, workspaces, invoices, roles and notify channels per test, so they run in
parallel against one database. Only `tests/seed.rs` uses the fixed seed rows.
