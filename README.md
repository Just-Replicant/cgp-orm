# cgp-orm

Transport-independent [SeaORM](https://www.sea-ql.org/SeaORM) repository and
resource-service components for
[Context-Generic Programming](https://contextgeneric.dev).

[![License: AGPL-3.0-or-later](https://img.shields.io/badge/license-AGPL--3.0--or-later-blue.svg)](LICENSE)
[![Tests](https://github.com/Just-Replicant/cgp-orm/actions/workflows/tests.yml/badge.svg)](https://github.com/Just-Replicant/cgp-orm/actions/workflows/tests.yml)
![Rust 1.99+](https://img.shields.io/badge/rustc-1.99+-blue.svg)

Design: [ARCHITECTURE.md](ARCHITECTURE.md). Wiring example:
[`tests/support/mod.rs`](tests/support/mod.rs). Changelog:
[CHANGELOG.md](CHANGELOG.md).

```sh
cargo test
cargo test --doc
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
```

## Recent API notes

These are described in full in the changelog. The ones that may be
**breaking** for existing callers:

- `ErrEntityNotFound` now has an `entity_name` field (`type_name` only).
  Use `ErrEntityNotFound::of::<E>()`.
- `Page::total` is `Option<u64>`. `PageRequest::without_total()` skips
  `COUNT(*)` and leaves `total` as `None`. `has_next` and `total_pages`
  return `Option` as well.
- `set_primary_key_on_active_model` returns a typed `PrimaryKeyWriteError`
  instead of panicking when `try_set` fails.
- `sqlite_unique_message` no longer matches the bare phrase
  `unique constraint`.

New, non-breaking for existing stack users:

- `CanBuildEntityFilter` is consulted by `paginate` (page and count) and
  `find_by_identity`. The default, `NoEntityFilter`, returns
  `Condition::all()`. Applications supply a tenant, search, or
  soft-delete filter; the library does not grow a `DeletionPolicy`.
- A real SQLite unique violation is classified by `DbErr::sql_err()`
  (`sql_err_is_unique` is asserted in `tests/sqlite.rs`).
- `OrUniqueConflict<A, B>` is wired as the provider type itself, not
  wrapped in `UseDelegate`.

Licensed under [AGPL-3.0-or-later](LICENSE).
