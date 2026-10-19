# AGENTS.md

Guidance for coding agents working on this repository. User-facing docs are
in `README.md` and `crates/air-tight/docs/guide.md` (the crate-level docs).
This file covers what you need to change the code safely.

## Layout

- `crates/air-tight-core`: traits, key newtypes, `Condition`, wildcard
  matching, `ResourceIds`, `ResourceKinds`. Must not depend on axum, diesel
  or moka.
- `crates/air-tight-macros`: proc macros only (derives, `#[permission]`,
  `#[unrestricted]`). Generated code refers to `::air_tight::...`, never to
  `air_tight_core`.
- `crates/air-tight`: runtime (client, cache, listener, management API,
  router, registry, extractors, `setup!`). Re-exports core and the macros.
- `examples/axum-diesel-app`: the integration test. It must compile after
  every change.

## Commands

All of these must pass before a commit:

```sh
cargo build --workspace
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps
AIR_TIGHT_TEST_DATABASE_URL=postgres://postgres:password@localhost:55432/air_tight_test \
  cargo test --workspace
```

Database tests skip themselves when `AIR_TIGHT_TEST_DATABASE_URL` is unset
and still count as passed, so a green run without it proves nothing about
the database code. Start Postgres with `docker compose up -d` (host port
55432; it also creates `air_tight_example` for the example app) or point
the variable at any database where you may create the `air_tight` schema.

## Tests

- Tests run in parallel against one shared database. Use fresh random user
  ids and role names (`common::unique`), give every test client its own
  notify channel (`t_<uuid>`, otherwise another test's `all` notification
  flushes your cache), and never truncate shared tables.
- Compile-fail tests live in `crates/air-tight/tests/ui/`, driven by
  `tests/derives.rs`. Regenerate the `.stderr` files with
  `TRYBUILD=overwrite cargo test -p air-tight --test derives ui`, then read
  every changed file: the message and the span are what is being tested.
  Rustc's "other types implement" hints change when new impls are added.
- Macro expansion snapshots live in `crates/air-tight-macros/tests/expand/`.
  Regenerate with `AIR_TIGHT_UPDATE_SNAPSHOTS=1 cargo test -p air-tight-macros`
  and review the diff.
- Inside `crates/air-tight` unit tests, the derives cannot be used (their
  `::air_tight::` paths do not resolve in the crate itself); hand-write trait
  impls there, or use the derives from `tests/`.

## Style

- One logical element per file, small files.
- Newtypes over raw strings and ids where natural.
- Every public item has a doc comment (`#![deny(missing_docs)]` is on).
- No em dashes anywhere: code, comments, docs, scripts.
- Errors go through the single `air_tight::Error` enum.

## Macro invariants (easy to break)

- `#[permission]` in handler mode moves the body into a hidden
  `__air_tight_handler_<name>` and reuses the handler's name for a factory
  returning `Guarded<impl Handler<(M, T1..Tn), S>>`. The typed routing
  functions accept only `Guarded`/`Public` via the sealed `RouteTarget`
  trait. Do not make plain handlers routable.
- Conditions compile to a `static Condition<Perms>` using the hidden
  `const fn __air_tight_lookup` (category derive) and `__air_tight_variant`
  (permission derive). These match Rust variant names, not key overrides.
  Changing either derive means checking the attribute macro too.
- The macros refer to `crate::__air_tight::{Perms, User}`, emitted by
  `setup!`. `setup!` resolves the user's types at the crate root under
  hidden alias names first, to avoid the aliases shadowing themselves.
- The injected user extractor is `CurrentUser<User>` so a missing principal
  is 401. Keep it that way.
- The registry matches routes by `std::any::type_name` of the factory with
  generics stripped against `module_path!()::name`. Renaming or wrapping the
  factory breaks `audit_routes`.

## Runtime invariants

- Every management mutation runs in one transaction, writes an audit row,
  sends `pg_notify` inside that transaction, then invalidates the local cache
  directly. Do not rely on the listener alone.
- Cache loads are guarded by generation counters (`generations.rs`) so a
  load racing an invalidation never leaves a stale set cached. Keep the
  bump-before-clear order.
- The listener thread holds one pooled connection and only a `Weak` to the
  client; pools need `max_size >= 2`.
- `Accessible` (the diesel filter) must bind every user-supplied value with
  `push_bind_param`. The only raw SQL it splices in is the id column.

## Open items

Do not implement these unless asked; they are marked in the code:

- `TODO(v2:role-hierarchy)` in `load_grant_set.rs`.
- `TODO(v2:multi-instance)` in `listener.rs` (documentation only).

The README section "Design notes and deviations from the original spec"
lists the deliberate departures from the original design. Read it before
"fixing" any of them.
