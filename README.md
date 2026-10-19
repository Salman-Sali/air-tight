# air-tight

Authorization for [axum](https://github.com/tokio-rs/axum) with Postgres-backed
grants. Declare permissions as Rust enums, protect handlers with one attribute,
manage roles and grants at runtime, and let a typed router guarantee that no
route ships unprotected.

```rust,ignore
#[air_tight::permission(Workspace(Update) && Invoice(Create), resources = [body])]
async fn create_invoice(Json(body): Json<CreateInvoice>) -> Result<StatusCode, ApiError> {
    // runs only if the caller may update this workspace and create invoices
}
```

## 60-second quickstart

1. Start Postgres (host port 55432) and run the example app:

   ```sh
   docker compose up -d
   cargo run -p axum-diesel-app
   curl -H "X-User-Id: aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa" http://127.0.0.1:3000/workspaces
   ```

   The app prints a list of curl commands on start; see
   [examples/axum-diesel-app](examples/axum-diesel-app/README.md).

2. Add the dependencies to your own app:

   ```toml
   [dependencies]
   air-tight = "0.1"
   axum = "0.8"
   diesel = { version = "2.2", features = ["postgres", "r2d2", "uuid"] }
   uuid = { version = "1", features = ["serde"] }
   ```

3. Declare permissions and resource kinds:

   ```rust,ignore
   use air_tight::{Permission, PermissionCategory, ResourceKind};

   #[derive(Permission)]
   pub enum WorkspaceAction { Read, Update }

   #[derive(ResourceKind)]
   pub enum MyResources { Workspace }          // also emits `my_resources_kinds::Workspace`
   use my_resources_kinds::Workspace;

   #[derive(PermissionCategory)]
   pub enum MyPerms {
       #[air_tight(resource = Workspace)]
       Workspace(WorkspaceAction),
   }
   ```

4. Implement `Principal` for your user type and register the types once in the
   crate root:

   ```rust,ignore
   impl air_tight::Principal for User {
       fn air_tight_id(&self) -> String { self.id.to_string() }
   }
   air_tight::setup!(perms = MyPerms, user = User, resources = MyResources);
   ```

5. Protect a handler (your auth middleware must insert the `User` into the
   request extensions):

   ```rust,ignore
   #[air_tight::permission(Workspace(Read))]
   async fn get_workspace(id: AirTightPath<Uuid, Workspace>) -> Json<Workspace> { .. }
   ```

6. Route it with the typed router, build the client (pool `max_size >= 2`) and
   audit the routes:

   ```rust,ignore
   let air_tight = AirTight::<MyPerms, User>::builder(pool)
       .resource_kinds::<MyResources>()
       .build()?;
   let router = air_tight::Router::new()
       .route("/workspaces/{id}", air_tight::routing::get(get_workspace))
       .layer(middleware::from_fn(authenticate));
   air_tight.audit_routes(&router);          // panics at startup on a gap
   let app = router.with_state(state).into_axum();
   ```

   Implement `FromRef<AppState>` for the client so handlers can extract it. The
   crate documentation (`cargo doc --open -p air-tight`) has the full
   walkthrough with a compiling example.

## Features

- Permissions as enums (`Category::Action` keys), wildcards (`*`, `Workspace::*`).
- Grants held globally or scoped to one resource `(kind, id)`, directly or
  through roles.
- Conditions with `&&`, `||`, `!`, `all(..)`, `any(..)` checked per request
  against the resources in the path, query and body (`AirTightPath`,
  `AirTightQuery`, `#[derive(IdentifyResources)]`).
- `#[permission]` for handlers and, with `kind = fn`, for service functions;
  `#[unrestricted]` for public handlers.
- A typed router that rejects unprotected handlers at compile time, plus a
  startup `audit_routes` check.
- `Granted` proof for in-memory re-checks with consistent grants.
- List filtering: `accessible_ids` (cached) and a diesel `accessible` filter (SQL).
- Management API (roles, assignments, grants, rename of permissions, reports)
  with a Postgres audit log.
- Per-user moka cache with idle expiry and LISTEN/NOTIFY invalidation, which
  also makes multiple application instances work with no extra setup.
- Embedded migrations in their own `air_tight` schema, with `migration_sql()`
  for self-managed setups.

## Workspace layout

| Path | Purpose |
| --- | --- |
| `crates/air-tight-core` | traits, keys, condition tree (no axum, diesel or moka) |
| `crates/air-tight-macros` | derives and attribute macros |
| `crates/air-tight` | runtime crate; re-exports the other two |
| `examples/axum-diesel-app` | runnable app and integration tests |

## Development

```sh
docker compose up -d
AIR_TIGHT_TEST_DATABASE_URL=postgres://postgres:password@localhost:55432/air_tight_test \
  cargo test --workspace
```

Without the variable the database tests are skipped.

## Design notes and deviations from the original spec

- `air_tight::Error` lives in the runtime crate, because core has no diesel
  dependency.
- Marker types are generated in a `<snake_case_enum>_kinds` module (for example
  `my_resources_kinds::Workspace`), because a module cannot share the enum's name.
- The catalog derives (`Permission`, `PermissionCategory`, `ResourceKind`) also
  implement `Clone`, `Copy`, `PartialEq`, `Eq`, `Hash` and `Debug`; do not derive
  them yourself.
- `ResourceIds` is implemented for common id types (`Uuid`, `String`, `&str`,
  integers, `char`) and for containers (`Option`, `Vec`, slices, sets) instead of
  a blanket `T: Display` impl, which would conflict with the container impls.
- Conditions are built in a `static` through hidden `const fn` lookups generated
  by the derives, so an unknown `Category(Action)` fails at compile time.
- In handler mode the function name becomes a factory returning
  `Guarded<impl Handler>`. Route it with `air_tight::routing::get(name)`; the
  handler is not callable directly.
- The injected user extractor is `CurrentUser<User>`, which reads the same
  request extension as `Extension<User>`, so a missing principal is a 401 rather
  than axum's 500.
- 403 response bodies do not list the missing keys; they are available on
  `Error::Forbidden`.
- `AirTightQuery` with bare tuples cannot be deserialized by `serde_urlencoded`;
  use a struct with `K = ()`.
- The LISTEN thread holds one pooled connection, so the pool needs
  `max_size >= 2`.
- The docker compose file maps Postgres to host port 55432 (to stay clear of a
  local Postgres on 5432).
- Scoped role semantics: a role assigned within `(Workspace, A)` grants its keys
  only for that exact scope. An `Invoice::Read` key inside a workspace-scoped
  role therefore does not grant reading invoices, because an `Invoice(..)` leaf
  checks `(Invoice, id)` scopes. The example seeds bob that way, as the spec
  asked, to show the effect; grant `Invoice::Read` globally or per invoice to
  change it.
- The management mutators take `actor: Option<&str>` as their first argument;
  no-op assign, grant and revoke calls write no audit row; `rename_permission`
  adds the `permission_renamed` audit action.

## License

MIT. See [LICENSE](LICENSE).
