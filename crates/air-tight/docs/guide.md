Authorization for [axum](https://docs.rs/axum) with Postgres-backed grants.

You declare permissions and resource kinds as Rust enums. Roles and grants live
in a Postgres schema called `air_tight`, are cached in memory per user, and are
kept fresh across processes with LISTEN/NOTIFY. Handlers are protected with one
attribute, `#[permission(...)]`, and a typed router refuses to route handlers
that are not protected (or explicitly marked `#[unrestricted]`).

This crate re-exports everything from `air-tight-core` and the macros from
`air-tight-macros`. Depend on `air-tight` only.

# Contents

1. [Concepts](#concepts)
2. [Declaring permissions and resources](#declaring-permissions-and-resources)
3. [`setup!` and the principal](#setup-and-the-principal)
4. [Authentication middleware](#authentication-middleware)
5. [Building the client](#building-the-client)
6. [Protecting handlers](#protecting-handlers)
7. [The typed router and `audit_routes`](#the-typed-router-and-audit_routes)
8. [Services: `kind = fn`](#services-kind--fn)
9. [`Granted::require`](#grantedrequire)
10. [Filtering lists](#filtering-lists)
11. [Management API and audit log](#management-api-and-audit-log)
12. [Cache and invalidation](#cache-and-invalidation)
13. [Migrations](#migrations)
14. [A complete example](#a-complete-example)

# Concepts

- A **permission** is a key `Category::Action`, for example `Workspace::Update`.
  Code defines them with two enums: a `Permission` enum of actions and a
  `PermissionCategory` enum whose variants wrap one action enum each.
- A **resource kind** is something grants can be scoped to, for example a
  workspace. Code defines them with a `ResourceKind` enum.
- A **grant** gives a user a permission key, either **globally** or within one
  **scope** `(kind, id)`, for example "`Workspace::Update` on workspace 42".
  Grants come directly (`grant`) or through **roles** (a named set of keys,
  assigned with `assign_role`, globally or within a scope).
- Keys may be wildcards: `*` matches everything and `Workspace::*` matches every
  action of the `Workspace` category.
- A **condition** combines permission leaves with `&&`, `||`, `!`, `all(..)` and
  `any(..)`. Each request has a list of **resource scopes** (the resources the
  request is about, taken from the path, the query and the body). A leaf whose
  category is bound to a resource kind is satisfied by a global grant or by a
  grant scoped to one of the request's scopes of that kind. A leaf of an
  unbound category (like `Billing` in the example) is satisfied by a global
  grant only.

# Declaring permissions and resources

```ignore
use air_tight::{Permission, PermissionCategory, ResourceKind};

#[derive(Permission)]
pub enum WorkspaceAction { Read, Update, Delete }

#[derive(Permission)]
pub enum BillingAction { Read, Manage }

#[derive(ResourceKind)]
pub enum MyResources { Workspace, Invoice }

// The derive above also emitted `my_resources_kinds::{Workspace, Invoice}`.
use my_resources_kinds::{Invoice, Workspace};

#[derive(PermissionCategory)]
pub enum MyPerms {
    #[air_tight(resource = Workspace)]
    Workspace(WorkspaceAction),
    #[air_tight(resource = Invoice)]
    Invoice(InvoiceAction),
    Billing(BillingAction), // no resource: always global
}
```

- All catalog derives also implement `Clone`, `Copy`, `PartialEq`, `Eq`, `Hash`
  and `Debug`. Do not derive those yourself.
- `#[derive(ResourceKind)]` on `MyResources` generates a module named
  `<snake_case_enum>_kinds` (here `my_resources_kinds`) with one marker type per
  variant. A module cannot share its enum's name, hence the suffix. The markers
  are what you put in `#[air_tight(resource = ...)]` and in
  `AirTightPath<_, Marker>`.
- Keys are derived from the identifiers: `MyPerms::Workspace(WorkspaceAction::Read)`
  is the key `Workspace::Read`. After renaming a variant, pin the old key with
  `#[air_tight(key = "...")]` on the variant, or migrate the stored data with
  [`AirTight::rename_permission`].
- Marking a category with `resource = Marker` binds it to that kind: its leaves
  are checked against the request's scopes of that kind. Without it, the
  category is global-only.
- `MyPerms::all()` lists every permission, and `from_key` parses a key.

# `setup!` and the principal

The principal is whatever represents the caller in your app. It only has to say
how to identify itself in the grant tables:

```ignore
use air_tight::Principal;

#[derive(Clone)]
pub struct AuthenticatedUser { pub id: UserId, pub name: String }

impl Principal for AuthenticatedUser {
    fn air_tight_id(&self) -> String { self.id.to_string() }
}
```

Then register the three types once, in the crate root (`main.rs` or `lib.rs`).
The attribute macros look them up there:

```ignore
air_tight::setup!(
    perms = authz::MyPerms,
    user = auth::AuthenticatedUser,
    resources = authz::MyResources
);
```

Forgetting it produces "could not find `__air_tight` in the crate root".

# Authentication middleware

air-tight does not authenticate anyone. Your middleware must insert the
principal into the request extensions:

```ignore
async fn authenticate(mut req: Request, next: Next) -> Response {
    let user = /* load the user from a header, session, JWT... */;
    req.extensions_mut().insert(user);
    next.run(req).await
}
```

Protected handlers read it back with [`CurrentUser`]`<User>`. When no principal
is present the request is answered with `401 Unauthorized` (not axum's `500`
for a missing extension).

# Building the client

```ignore
let pool = Pool::builder().max_size(8).build(ConnectionManager::<PgConnection>::new(url))?;

let air_tight = AirTight::<MyPerms, AuthenticatedUser>::builder(pool.clone())
    .resource_kinds::<MyResources>()   // required, see below
    .cache_idle_ttl(Duration::from_secs(600))   // optional
    .notify_channel("air_tight_invalidate")     // optional
    .build()?;
```

- `builder` returns a [`Builder`] whose `R` type parameter starts as [`NoKinds`].
  `build()` only exists after `.resource_kinds::<R>()`, so forgetting it is a
  compile error (a typestate).
- `build()` runs the embedded migrations, syncs the permission and resource kind
  catalog to the database, starts the LISTEN thread and returns the client.
- **The pool needs a `max_size` of at least 2**: the LISTEN thread keeps one
  pooled connection for as long as the client lives.
- The notify channel must be a plain lowercase identifier. All instances that
  should see each other's changes must use the same channel and database.
- Other options: `cache_max_bytes` and `cache` (a custom [`GrantCache`]).

The client is cheap to clone and `Send + Sync`. Put it in your app state and
implement `FromRef` so handlers can extract it:

```ignore
#[derive(Clone)]
struct AppState { db: Db, air_tight: AirTight<MyPerms, AuthenticatedUser> }

impl FromRef<AppState> for AirTight<MyPerms, AuthenticatedUser> {
    fn from_ref(state: &AppState) -> Self { state.air_tight.clone() }
}
```

# Protecting handlers

```ignore
#[air_tight::permission(Workspace(Update))]
async fn update_workspace(
    State(db): State<Db>,
    id: AirTightPath<Uuid, Workspace>,
    Json(body): Json<Rename>,
) -> Result<Json<WorkspaceRecord>, ApiError> {
    // runs only if the caller holds Workspace::Update on this workspace
}
```

**Condition grammar.** `Category(Action)` is a leaf. `Category(Read && (Update || Delete))`
is shorthand for the same category. Combine leaves with `&&`, `||`, `!` (the user
must *lack* the permission), parentheses, `all(a, b, ..)` and `any(a, b, ..)`.
Examples: `Workspace(Update) && Invoice(Create)`, `all(Invoice(Update), Workspace(Update))`,
`Billing(Read) && !Billing(Manage)`. An unknown `Category(Action)` is a compile
error.

**Resources.** The request's scopes come from parameters that implement
`IdentifyResources`:

- Parameters typed `AirTightPath<..>` or `AirTightQuery<..>` are included
  automatically. `AirTightPath<Uuid, Workspace>` is a path extractor whose
  value is a workspace id; tuples work too:
  `AirTightPath<(Uuid, Uuid), (Workspace, Invoice)>`.
- Any other parameter is listed with `resources = [name, ..]`, using its binding
  name, either plain (`body`) or one level deep (`Json(body)` yields `body`).
  Derive `IdentifyResources` on the type and mark the id fields:

```ignore
#[derive(Deserialize, IdentifyResources)]
pub struct CreateInvoice {
    #[air_tight(resource = Workspace)]
    pub workspace_id: Uuid,
    pub amount: i64,
}

#[air_tight::permission(Workspace(Update) && Invoice(Create), resources = [body])]
async fn create_invoice(Json(body): Json<CreateInvoice>) -> Result<StatusCode, ApiError> { .. }
```

  Fields can be single ids, `Option`, `Vec` or other [`ResourceIds`]; use
  `#[air_tight(nested)]` for a field that itself implements `IdentifyResources`.
  Nothing is guessed: only marked fields contribute scopes. A `Vec` of ids means
  the condition must hold for every one of them.
- `AirTightQuery<T, K>` works like `AirTightPath` for the query string. Use a
  struct deriving `IdentifyResources` with `K = ()`; bare tuples cannot be
  deserialized from a query string.

**What the macro does.** In handler mode the function name becomes a *factory*
returning `Guarded<impl Handler>`. Route it with `air_tight::routing::get(name)`;
the handler cannot be called directly. The original body moves into a hidden
function. The macro injects `State<AirTight<Perms, User>>` and
[`CurrentUser`]`<User>` parameters, authorizes before the body runs, and makes a
`granted: Granted<Perms>` binding available inside the body.

**Error modes.** If the return type is syntactically `Result<_, _>`, failures are
returned with `?`, so your error type needs `From<air_tight::Error>`. Otherwise
the failure is turned into a response with `IntoResponse` and the return type
must be `Response` or `impl IntoResponse`. Override the detection with
`error = result` or `error = response`.
[`Error`] maps to `401` (`Unauthenticated`), `403` (`Forbidden`) and `500`
(everything else). The `403` body does not list the missing keys; they are on
`Error::Forbidden { missing }` if you want to log or render them yourself.

**Your own state and user parameters.** If you already extract the client or the
user, name your parameters instead of having them injected:

```ignore
#[air_tight::permission(Workspace(Update), state = client, user = me)]
async fn rename(
    State(client): State<Client>,
    CurrentUser(me): CurrentUser<AuthenticatedUser>,
    id: AirTightPath<Uuid, Workspace>,
) -> Result<(), ApiError> { /* client and me are yours to use */ }
```

**Public handlers.** `#[air_tight::unrestricted]` marks a handler that needs no
permission (a health check, or a handler that filters its own output):

```ignore
#[air_tight::unrestricted]
async fn health() -> &'static str { "ok" }
```

# The typed router and `audit_routes`

[`Router`] wraps `axum::Router`. Its `route` only accepts method routers built
with [`routing`] (`get`, `post`, `put`, `patch`, `delete`, ...) from handlers
processed by `#[permission]` or `#[unrestricted]`. A plain axum handler does not
compile:

```ignore
let router = air_tight::Router::new()
    .route("/workspaces/{id}", routing::get(get_workspace).patch(update_workspace))
    .route("/health", routing::get(health))
    .layer(middleware::from_fn(authenticate));

air_tight.audit_routes(&router);              // panics at startup on a gap
let app: axum::Router = router.with_state(state).into_axum();
```

`audit_routes` checks every recorded route against the registry that the macros
fill, and panics naming the method and path if one is missing. Plain
`axum::Router`s converted with `Router::from` and merged or nested in bypass the
gate: their routes are not recorded, so neither the compiler nor the audit
covers them. Build everything that needs authorization with the typed router.

# Services: `kind = fn`

The same attribute protects ordinary functions, so the rule travels with the
function wherever it is called from:

```ignore
#[air_tight::permission(Workspace(Update), kind = fn, resources = [workspace])]
pub async fn rename_workspace(
    db: &Db,
    workspace: WorkspaceRef,
    name: String,
) -> Result<WorkspaceRecord, ApiError> { .. }

// The macro prepends (&AirTight, &User, Vec<ResourceScope>) to the parameters.
rename_workspace(&client, &me, Vec::new(), &db, workspace, name).await?;
```

The extra `Vec<ResourceScope>` lets the caller add scopes to those derived from
`resources`. `async` functions use `authorize_async`, others `authorize`. A
`granted` binding is available here too. `error = response` is not allowed in
function mode.

# `Granted::require`

[`Granted`] is the proof that a request passed its check. It carries a snapshot
of the user's grants, so further checks are in memory, consistent with the first
one, and never touch the database:

```ignore
static MANAGE: Condition<MyPerms> = Condition::Leaf(MyPerms::Billing(BillingAction::Manage));

#[air_tight::permission(Billing(Read))]
async fn billing() -> Json<Summary> {
    Json(Summary { can_manage: granted.require(&MANAGE).is_ok() })
}
```

# Filtering lists

Row-level lists need the set of ids a user may see, not a yes or no per row.

- [`AirTight::accessible_ids`] answers from the cache with an [`Access`]:
  `All` (a global grant or wildcard), `Some(ids)` or `None`.

```ignore
let access = client.accessible_ids(&user_id, MyResources::Workspace.key(),
                                   MyPerms::Workspace(WorkspaceAction::Read))?;
```

- [`AirTight::accessible`] (boxed) and [`AirTight::accessible_expr`] (concrete
  [`Accessible`]) build a diesel filter that runs the check inside the query. It
  reads the grant tables directly, bypassing the cache:

```ignore
workspaces::table
    .filter(client.accessible(&user_id, MyResources::Workspace.key(),
                              MyPerms::Workspace(WorkspaceAction::Read), workspaces::id))
    .load(conn)?
```

Use `accessible_ids` for small id sets and cache speed, the SQL expression for
large tables or when you need always-fresh data.

# Management API and audit log

All mutators are methods on [`AirTight`], run in one transaction, write an audit
row, send a NOTIFY and invalidate the local cache. The first argument is
`actor: Option<&str>`, recorded in the audit log (`None` for system actions).

| Method | Audit action |
| --- | --- |
| `create_role(actor, &RoleName, description)` | `role_created` |
| `delete_role(actor, RoleId)` | `role_deleted` |
| `set_role_permissions(actor, RoleId, &[PermissionKey])` | `role_permissions_set` |
| `assign_role(actor, user, RoleId, Option<ResourceScope>)` | `role_assigned` |
| `revoke_role(actor, user, RoleId, Option<ResourceScope>)` | `role_revoked` |
| `grant(actor, user, PermissionKey, Option<ResourceScope>)` | `grant_added` |
| `revoke(actor, user, PermissionKey, Option<ResourceScope>)` | `grant_revoked` |
| `rename_permission(actor, &old, &new)` | `permission_renamed` |

Assigning, granting or revoking something that is already (or not) in place is
an idempotent no-op: it writes no audit row and sends no notification. Keys
given to `set_role_permissions` and `grant` must be in the catalog or be
well-formed wildcards, else `Error::Catalog`.

Read side: [`AirTight::effective_grants`] (everything a user holds, with the
source: direct or role name), [`AirTight::users_with_access`],
[`AirTight::list_roles`], [`AirTight::list_permissions`]. These read the
database, not the cache. The audit rows are in `air_tight.audit_log`.

**Scoped role semantics.** A role assigned within a scope grants its keys *only
for that exact scope*. If the role `Member` holds `Workspace::Read` and
`Invoice::Read` and is assigned to a user within `(Workspace, A)`, the user can
read workspace A, but `Invoice::Read` does not let them read invoices: an
`Invoice(..)` leaf checks scopes of kind `Invoice`, and the grant is scoped to a
workspace. To grant invoice access, assign the role (or grant the key) globally
or within the invoice scopes.

# Cache and invalidation

Each user's flattened grants are cached in a moka cache (time to idle 10
minutes, 256 MiB by default; see [`MokaGrantCache`]). A miss loads the grants
with two queries; `authorize_async` keeps that off the async executor.

Every management write sends `pg_notify` on the channel in the same transaction
(payloads `user:<id>`, `role:<uuid>` or `all`), so it is delivered on commit. A
listener thread per client receives them and invalidates the matching cache
entries. After a reconnect (with exponential backoff) the whole cache is
dropped, because notifications may have been missed.

**Multiple instances need nothing extra.** Because invalidation travels through
Postgres, any number of processes sharing a database and a notify channel
invalidate each other's caches, with the same code path a single instance uses
for its own writes. (Project note `TODO(v2:multi-instance)`: documentation only,
no work planned.) If you change the tables behind air-tight's back (raw SQL),
call [`AirTight::invalidate_user`], [`AirTight::invalidate_role`] or
[`AirTight::invalidate_all`] yourself.

Role inheritance (`role_parents`) is not implemented yet
(`TODO(v2:role-hierarchy)`); roles are flat sets of keys.

# Migrations

`build()` applies the embedded migrations to the `air_tight` schema, tracked in
its own `air_tight.migrations` table, under an advisory lock, so it is safe to
start many instances at once and it never touches diesel's migration table.

If you manage migrations yourself (for example in a deployment step), call
[`run_migrations`] with a connection, or take the SQL with [`migration_sql`]
(or `AirTight::migration_sql()`), which returns `(version, sql)` pairs in order.

# A complete example

This compiles (it is not run, because it needs a database):

```no_run
use std::time::Duration;

use air_tight::{
    AirTight, AirTightPath, CurrentUser, Permission, PermissionCategory, Principal,
    ResourceKind, routing,
};
use axum::extract::{FromRef, Request};
use axum::middleware::{self, Next};
use axum::response::Response;
use axum::Json;
use diesel::PgConnection;
use diesel::r2d2::{ConnectionManager, Pool};
use uuid::Uuid;

#[derive(Permission)]
pub enum WorkspaceAction { Read, Update }

#[derive(ResourceKind)]
pub enum Resources { Workspace }

use resources_kinds::Workspace;

#[derive(PermissionCategory)]
pub enum Perms {
    #[air_tight(resource = Workspace)]
    Workspace(WorkspaceAction),
}

#[derive(Clone)]
pub struct User { pub id: Uuid }

impl Principal for User {
    fn air_tight_id(&self) -> String { self.id.to_string() }
}

air_tight::setup!(perms = Perms, user = User, resources = Resources);

#[derive(Clone)]
struct AppState { air_tight: AirTight<Perms, User> }

impl FromRef<AppState> for AirTight<Perms, User> {
    fn from_ref(state: &AppState) -> Self { state.air_tight.clone() }
}

async fn authenticate(mut req: Request, next: Next) -> Response {
    req.extensions_mut().insert(User { id: Uuid::nil() });
    next.run(req).await
}

#[air_tight::permission(Workspace(Read))]
async fn get_workspace(id: AirTightPath<Uuid, Workspace>) -> Json<String> {
    Json(id.to_string())
}

#[air_tight::permission(Workspace(Update), state = client, user = me)]
async fn rename_workspace(
    axum::extract::State(client): axum::extract::State<AirTight<Perms, User>>,
    CurrentUser(me): CurrentUser<User>,
    id: AirTightPath<Uuid, Workspace>,
) -> Json<String> {
    let _ = (&client, &me);
    Json(id.to_string())
}

#[air_tight::unrestricted]
async fn health() -> &'static str { "ok" }

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let url = "postgres://postgres:password@localhost:55432/air_tight_example";
    let pool = Pool::builder()
        .max_size(8) // at least 2: the listener holds one connection
        .build(ConnectionManager::<PgConnection>::new(url))?;

    let air_tight = AirTight::<Perms, User>::builder(pool)
        .resource_kinds::<Resources>()
        .cache_idle_ttl(Duration::from_secs(600))
        .build()?;

    let router = air_tight::Router::<AppState>::new()
        .route(
            "/workspaces/{id}",
            routing::get(get_workspace).patch(rename_workspace),
        )
        .route("/health", routing::get(health))
        .layer(middleware::from_fn(authenticate));
    air_tight.audit_routes(&router);

    let _app: axum::Router = router.with_state(AppState { air_tight }).into_axum();
    // Serve `_app` with axum::serve.
    Ok(())
}
```

The runnable version of all of this, with seed data, services, list filtering,
management endpoints and integration tests, is in `examples/axum-diesel-app`
of the repository.
