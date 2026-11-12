# Changelog

## [Unreleased]

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

No deprecated aliases are provided.

### Changed

- `get_or_create` loads the existing row before inserting. A repeat call no
  longer attempts an insert, so it does not consume an autoincrement id.
  A concurrent unique conflict still rolls back and fetches the winner.
- `upsert` resolves a concurrent unique conflict on a new transaction after
  rollback. The follow-up select no longer runs on the transaction the
  failed insert aborted.

### Documentation

- Rewrote the README: features, installation, a quick-start that uses
  `get_by_id` and `get_or_create`, MSRV, testing, and license.
