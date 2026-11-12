# cgp-orm

Transport-independent [SeaORM](https://www.sea-ql.org/SeaORM) repository and
resource-service components for
[Context-Generic Programming](https://contextgeneric.dev)
([`cgp-fork`](https://crates.io/crates/cgp-fork) 1.1).

**Presets compose behavior, contexts choose policies, and macros do not own
architecture.**

This crate is reusable and intended to be published. It depends only on
`cgp-fork`, `sea-orm`, and `thiserror`. It contains no transport stack,
no application entities, no anyhow, and no procedural macros.

Full design: [ARCHITECTURE.md](ARCHITECTURE.md).

## What you get

- **Primary keys from SeaORM.** Repository methods take
  `PrimaryKeyOf<C>` — SeaORM's complete `PrimaryKeyTrait::ValueType`.
  `i32`, `Uuid`, and composite keys work with no `From<i32>` bound.
- **Repository layer.** `CanFindEntityById`, `CanInsertEntity`,
  `CanUpdateEntityById`, `CanDeleteEntityById`, `CanFindEntityByIdentity`,
  `CanPaginateEntities`, plus explicit writes: `insert`, `find_by_identity`,
  `insert_or_get`, `upsert`.
- **Service layer.** Associated types `CreateInput`, `UpdateInput`, and
  `Output` (not a fixed DTO), with `CanDecodeCreateInput`,
  `CanApplyUpdateInput`, `CanEncodeOutput`, `CanCreateResource`,
  `CanGetOrCreateResource`, `CanUpdateResource`, `CanGetResource`,
  `CanListResources`, and `CanDeleteResource`.
- **Identity policy.** `CanBuildIdentityCondition` replaces
  `fn unique_columns(&self) -> Vec<Column>` as the public API.
  `UniqueColumnsIdentity` is the reusable column-list provider; an entity
  may supply a more complex identity.
- **Correct write semantics.** There is no generic `find_or_insert` that
  implies atomicity. `insert_or_get` relies on a **database unique
  constraint** and handles the conflict explicitly. A transaction around
  find-then-insert is not treated as sufficient.
- **Composite keys.** There is no `primary_key_column()` that returns only
  the first key. `find_by_id` uses the full `ValueType`. Update merging
  excludes **every** primary-key column.
- **Pagination.** `PageRequest { page, size }` with a configurable default
  and maximum (crate default: 20 / 100). Deterministic ordering is
  required (default: all primary-key columns). Page size is never
  hardcoded to 5. Cursor/keyset pagination is a documented extension
  point, not implemented here.
- **Transactions.** Completion preserves the original operation error if
  rollback also fails.
- **Stacks.** `RepositoryLookupStack`, `RepositoryCrudStack`, and
  `ResourceServiceStack`.

## Quick start

```rust,ignore
use cgp_orm::prelude::*;
use sea_orm::DatabaseConnection;

cgp_preset! {
    PostBase: RepositoryCrudStack + ResourceServiceStack {
        EntityTypeProviderComponent: UseType<post::Entity>,
        CreateInputTypeProviderComponent: UseType<CreatePost>,
        UpdateInputTypeProviderComponent: UseType<UpdatePost>,
        OutputTypeProviderComponent: UseType<PostOutput>,
        // codecs, identity, and hooks — chosen by the application
    }
}

#[derive(HasField, Clone)]
pub struct PostApp {
    pub db_conn: DatabaseConnection,
}
```

Wire an error map on a child preset (`ErrorType`, `ErrorRaiser`,
`ErrorWrapper`). How those map into the application's error type is
**application policy** and lives in the consuming crate.

The context must provide:

- `HasEntityType` — the SeaORM entity.
- `HasUniqueColumns` — if you use the default `UniqueColumnsIdentity`.
- Codecs and the three service associated types — if you use
  `ResourceServiceStack`.
- An error raiser for `sea_orm::DbErr` and the crate's typed source
  errors (`ErrIdentityEmpty`, `ErrIdentityFieldUnset`,
  `ErrMissingOrdering`, `ErrConflictFetchFailed`, and the others in
  [`error`](crate::error)).

## Write operations

| Method | Behavior |
| --- | --- |
| `insert` | Insert. Unique conflicts are errors. Runs the write hook in a transaction. |
| `find_by_identity` | Select one row using the identity policy. |
| `insert_or_get` | Insert; on a unique-constraint conflict, roll back and fetch by identity. |
| `upsert` | Find by identity; insert or merge-update. Handles a concurrent unique conflict. |
| `update_by_id` | Look up by the complete primary key; merge non-key columns only. |
| `delete_by_id` | Delete by the complete primary key; returns `bool`. |

`insert_or_get` requires a real unique constraint in the database that
matches the identity policy. Without one, two concurrent inserts can
both succeed.


## Unique-conflict detection

`insert_or_get` and `upsert` ask a **pluggable policy**
(`CanDetectUniqueConflict`) whether a `DbErr` is a unique-constraint
violation. The lookup stack defaults to `StandardUniqueConflict`:

1. SeaORM's structured `DbErr::sql_err()` classifier (accurate when a
   `sqlx-*` or `rusqlite` feature is enabled on `sea-orm`).
2. `DbErr::RecordNotInserted` (`ON CONFLICT DO NOTHING`).
3. A conservative message fallback for PostgreSQL (`23505`), MySQL
   (`Duplicate entry` / `error 1062`), and SQLite (`UNIQUE constraint
   failed`). The fallback does **not** match foreign-key or check
   constraints.

Override `UniqueConflictDetectorComponent` for a tighter policy:
`SqlErrUniqueConflict` (structured only), `SqliteUniqueConflict`,
`PostgresUniqueConflict`, `MysqlUniqueConflict`,
`MessageUniqueConflict`, or `OrUniqueConflict<A, B>`.

## Pagination

```rust,ignore
let page = app.paginate(PageRequest::new(1, 50)).await?;
// page.items, page.page, page.size (clamped), page.total
```

- `page` is 1-based; `0` is treated as `1`.
- `size == 0` selects the policy default, itself clamped to the maximum.
- A requested size above the maximum is clamped.
- An empty ordering policy is an error (`ErrMissingOrdering`).

Override `PaginationPolicyComponent` for different limits (the default
provider is `DefaultPaginationPolicy`: 20 / 100). Override
`EntityOrderingComponent` for a non-PK order; it must still be
deterministic.

Cursor pagination can be added later as `CursorRequest { after, size }`
beside `PageRequest` without changing this API. See `ARCHITECTURE.md`.

## License

Licensed under the GNU Affero General Public License v3.0 or later
([AGPL-3.0-or-later](LICENSE)).
