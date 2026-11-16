# Changelog

## [Unreleased]

### Breaking

- `HasUniqueColumns::unique_columns` now returns
  `&'static [ColumnOf<Self>]` instead of `Vec<ColumnOf<Self>>`. The list
  is a static context policy of `Copy` column enums, so building an
  identity condition no longer allocates a column list. Change providers
  from `vec![Column::Name]` to `&[Column::Name]`. `HasUniqueColumns` is
  now declared with `#[cgp_component(UniqueColumnsGetter)]` instead of
  `#[cgp_getter]` (a `'static` slice is not a field borrow); the provider
  trait `UniqueColumnsGetter` and the key `UniqueColumnsGetterComponent`
  keep their names, so existing wiring is unchanged. The public identity
  API is still `CanBuildIdentityCondition`.
- Removed `util::value_tuple_from_vec` and `util::value_tuple_to_vec`.
  The primary-key helpers now match `ValueTuple::One` / `Two` / `Three`
  directly and consume the `Vec` that `ValueTuple::Many` already owns.
- Removed `PrimaryKeyReadError::Arity`. Reading a key can no longer fail
  on arity (an empty key is `PrimaryKeyReadError::Missing`), so
  `UpdateEntityById` no longer requires `CanRaiseError<ErrUnsupportedKeyArity>`.
  `PrimaryKeyWriteError::Arity` is unchanged and still reports a key value
  whose arity does not match the entity.

### Changed

- `get_or_create` no longer clones the active model. The identity lookup
  borrows the model, so the common "row already exists" path copies no
  `Set` strings or blobs. On a miss the identity `Condition` (just the
  identity values) is built before the insert consumes the model, and a
  unique-conflict refetch reuses that condition, ANDed with the entity
  filter, on a clean connection. `GetOrCreateEntity` no longer requires
  `ActiveModel: Clone`. `upsert` still clones once, because the conflict
  path needs the full model for `merge_update`.
- `is_primary_key_column`, `merge_update`, `primary_key_from_active_model`,
  and `set_primary_key_on_active_model` walk `PrimaryKey::iter()` in place
  instead of collecting `primary_key_columns()` into a `Vec` (the empty-key
  check is `iter().next().is_none()`). Columns are still compared by
  `as_str()`, so column enums need no `PartialEq` bound. Keys of arity
  1–3 are read and written without any intermediate `Vec`.
- The unique-conflict message fallback renders the `DbErr` once and scans
  that single string with an ASCII case-insensitive substring check. The
  PostgreSQL, MySQL, and SQLite matchers no longer each build a lowercased
  copy. The structured `sql_err()` and `RecordNotInserted` checks still
  return before any string is formatted.

## [0.1.2](https://github.com/Just-Replicant/cgp-orm/compare/v0.1.1...v0.1.2) - 2026-11-12

### Added

- *(pagination)* optional COUNT and apply entity filter
- *(filter)* add entity filter component and default

### Documentation

- document filter, optional totals, and hardening

### Fixed

- *(conflict)* tighten sqlite unique matcher
- *(error)* name not-found and typed PK set failure

### Other

- Merge branch 'feature/conflict-filter-hardening'

## [0.1.1](https://github.com/Just-Replicant/cgp-orm/compare/v0.1.0...v0.1.1) - 2026-11-12

### Changed

- *(service)* create via get_or_create
- *(repository)* rename insert_or_get to get_or_create
- *(repository)* rename find_by_id to get_by_id

### Documentation

- document get_by_id and get_or_create API

### Fixed

- *(repository)* resolve upsert conflict on a fresh transaction

### Breaking

- Renamed the repository method `find_by_id` to `get_by_id`. The consumer
  trait is now `CanGetEntityById` (was `CanFindEntityById`), the default
  provider is `GetEntityById` (was `FindEntityById`), and the component key
  is `EntityByIdGetter` (was `EntityByIdFinder`).
- Renamed the repository method `insert_or_get` to `get_or_create`. The
  consumer trait is now `CanGetOrCreateEntity` (was `CanInsertOrGetEntity`),
  the default provider is `GetOrCreateEntity` (was `InsertOrGetEntity`), and
  the component key is `EntityGetOrCreator` (was `EntityInsertOrGetter`).
- Removed the resource service `CanGetOrCreateResource`. `CanCreateResource::create`
  now delegates to `CanGetOrCreateEntity::get_or_create` and returns
  `(Output, InsertOutcome)`.
- `ErrEntityNotFound` now carries `entity_name: &'static str` (from
  `core::any::type_name` only, so the type stays lifetime-free), the same
  way `ErrMissingPrimaryKey` already does. Construct it with
  `ErrEntityNotFound::of::<E>()`. The Display message is
  `entity {entity_name} not found`. Direct construction of the former unit
  struct no longer compiles.
- `Page::total` is now `Option<u64>`. It is `Some` when the request asks
  for a count (the default) and `None` when `PageRequest::without_total`
  skips `COUNT(*)`. `Page::has_next` now returns `Option<bool>` and
  `Page::total_pages` returns `Option<u64>`; both are `None` when the
  total is unknown. `Page::maybe_has_next` offers a full-page heuristic
  for that case.
- `PageRequest` has a new public field, `include_total` (default `true`
  from `PageRequest::new` and `PageRequest::first`). Struct literals that
  name only `page` and `size` must also set this field, or use the
  constructors. `ResolvedPage` carries the same flag.
- `set_primary_key_on_active_model` no longer panics when `try_set`
  fails. It returns `PrimaryKeyWriteError` (was `PrimaryKeyReadError`),
  whose `SetFailed` variant wraps the new `ErrPrimaryKeySetFailed` source
  error. Callers raise it through `CanRaiseError`. The
  `expect("arity checked")` in `value_tuple_from_vec` is unchanged.
- `sqlite_unique_message` no longer matches the bare token
  `unique constraint`. The documented phrase `unique constraint failed`
  is still required and is sufficient.

No deprecated aliases are provided.

### Added

- `CanBuildEntityFilter`, a CGP component consulted by `paginate` (both
  the page query and the count) and by `find_by_identity`. The default
  provider, `NoEntityFilter`, returns `Condition::all()`, so every
  existing context stays valid. Applications supply a filter for tenant
  isolation, search, or soft delete. The library does not grow a
  `DeletionPolicy`.
- `PageRequest::without_total` (and `with_total`) to skip `COUNT(*)` and
  return `Page { total: None, .. }`. Including the total remains the
  default. Cursor pagination is still not implemented.
- `ErrPrimaryKeySetFailed`, a typed source error for a `try_set` failure
  when writing a primary key onto an active model.

### Changed

- `get_or_create` loads the existing row before inserting. A repeat call no
  longer attempts an insert, so it does not consume an autoincrement id.
  A concurrent unique conflict still rolls back and fetches the winner.
- `upsert` resolves a concurrent unique conflict on a new transaction after
  rollback. The follow-up select no longer runs on the transaction the
  failed insert aborted.

### Tests

- A real SQLite unique-constraint failure in `tests/sqlite.rs` asserts
  that `sql_err_is_unique` returns `true`. The structured `DbErr::sql_err()`
  path is what `StandardUniqueConflict` prefers; a SeaORM regression
  cannot silently fall through to the message fallback on the path this
  suite hits. No `sqlx-conflict` feature is introduced.
- `OrUniqueConflict<SqlErrUniqueConflict, MessageUniqueConflict>` is
  composed in a unit test and wired directly (not through `UseDelegate`)
  on a SQLite test context.

### Documentation

- Rewrote the README: features, installation, a quick-start that uses
  `get_by_id` and `get_or_create`, MSRV, testing, and license.
- Documented that `RecordNotInserted` is a step of `StandardUniqueConflict`.
  Applications that never use `ON CONFLICT DO NOTHING` opt out by wiring
  `SqlErrUniqueConflict` or a backend policy. `TreatRecordNotInsertedAsConflict`
  is not added.
- The `OrUniqueConflict` example now shows the wiring that compiles
  (`OrUniqueConflict<A, B>` as the provider type) and is a real doctest.
- Documented `CanBuildEntityFilter` and that soft delete is an
  application filter, not a library deletion policy. Noted the
  breaking `ErrEntityNotFound` and `Page::total` changes.
