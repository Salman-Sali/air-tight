# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - Unreleased

### Added

- `air-tight-core`: `PermissionKey`, `ResourceKindKey`, `ResourceScope`,
  `Principal`, the `Permission`, `PermissionCategory`, `ResourceKind`,
  `IdentifyResources` and `ResourceIds` traits, and the `Condition` tree with
  wildcard matching.
- `air-tight-macros`: derives for `Permission`, `PermissionCategory`,
  `ResourceKind` (with a `<snake_enum>_kinds` marker module) and
  `IdentifyResources`; the `#[permission]` attribute (handler mode and
  `kind = fn` service mode, condition grammar, automatic `AirTightPath` and
  `AirTightQuery` scopes, `error`, `state` and `user` overrides, `granted`
  binding) and `#[unrestricted]`; compile-time validation with `trybuild` tests.
- `air-tight`: the `AirTight` client and typestate `Builder`, `authorize`,
  `authorize_async`, `check`, `Granted::require`, `setup!`.
- Postgres schema `air_tight` with embedded migrations, their own tracking
  table, `run_migrations` and `migration_sql`, and catalog synchronisation.
- Per-user grant cache (`GrantCache`, moka implementation with idle expiry and
  a byte weigher) and precise invalidation by user, role or everything.
- LISTEN/NOTIFY invalidation with reconnect and backoff; multiple application
  instances stay consistent without extra setup.
- Management API: roles, assignments, grants, permission rename, effective
  grants, `users_with_access`, role and permission listings, and an audit log.
- Typed `Router` and `routing` functions that only accept protected or
  unrestricted handlers, handler registry and `audit_routes`.
- `AirTightPath` and `AirTightQuery` extractors, `CurrentUser`.
- List filtering with `accessible_ids` (cache) and the diesel `accessible`
  expression (SQL).
- `examples/axum-diesel-app` with seed data, a curl script and integration tests.
- Crate documentation with a complete walkthrough, and a root README.

### Notes

- Role inheritance is not implemented (`TODO(v2:role-hierarchy)`).
