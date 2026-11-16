//! Pluggable unique-conflict detection policies.
//!
//! [`CanDetectUniqueConflict`] is a context-chosen policy. The lookup stack
//! defaults to [`StandardUniqueConflict`], which tries SeaORM's structured
//! [`DbErr::sql_err`] classifier first (accurate when a sqlx or rusqlite
//! backend feature is enabled on `sea-orm` through feature unification),
//! then [`DbErr::RecordNotInserted`], then a conservative message fallback.
//!
//! Applications that target a single backend, or that can enable sqlx,
//! should consider a tighter policy:
//!
//! - [`SqlErrUniqueConflict`] — structured classifier only, no message match.
//! - [`SqliteUniqueConflict`], [`PostgresUniqueConflict`],
//!   [`MysqlUniqueConflict`] — backend-specific message patterns.
//! - [`MessageUniqueConflict`] — the multi-backend message fallback alone.
//! - [`OrUniqueConflict<A, B>`] — disjunction of two policies.
//!
//! Override `UniqueConflictDetectorComponent` on the application preset.

use core::marker::PhantomData;

use cgp_fork::prelude::*;
use sea_orm::{DbErr, SqlErr};

use crate::components::conflict::{UniqueConflictDetector, UniqueConflictDetectorComponent};

/// Structured SeaORM classification via [`DbErr::sql_err`].
///
/// This is accurate for unique and primary-key violations on MySQL,
/// PostgreSQL, and SQLite **when** the corresponding `sqlx-*` (or
/// `rusqlite`) feature is enabled on `sea-orm`. Without those features,
/// [`DbErr::sql_err`] always returns `None` and this policy never
/// matches — pair it with a message policy, or use
/// [`StandardUniqueConflict`].
pub struct SqlErrUniqueConflict;

#[cgp_impl(SqlErrUniqueConflict)]
impl UniqueConflictDetector {
    fn is_unique_conflict(&self, err: &DbErr) -> bool {
        sql_err_is_unique(err)
    }
}

/// Message-based multi-backend fallback.
///
/// Matches documented PostgreSQL, MySQL, and SQLite unique-violation
/// phrases. It does **not** match foreign-key or check-constraint
/// failures. Message matching is a heuristic: prefer
/// [`SqlErrUniqueConflict`] when a sqlx backend is available, or a
/// backend-specific policy when you target one database.
pub struct MessageUniqueConflict;

#[cgp_impl(MessageUniqueConflict)]
impl UniqueConflictDetector {
    fn is_unique_conflict(&self, err: &DbErr) -> bool {
        message_is_unique(&err.to_string())
    }
}

/// SQLite message patterns only (`UNIQUE constraint failed`,
/// `SQLITE_CONSTRAINT_UNIQUE`, `PRIMARY KEY constraint failed`).
pub struct SqliteUniqueConflict;

#[cgp_impl(SqliteUniqueConflict)]
impl UniqueConflictDetector {
    fn is_unique_conflict(&self, err: &DbErr) -> bool {
        sql_err_is_unique(err) || sqlite_unique_message(&err.to_string())
    }
}

/// PostgreSQL message patterns only (SQLSTATE `23505`,
/// `unique violation`, `duplicate key value`).
pub struct PostgresUniqueConflict;

#[cgp_impl(PostgresUniqueConflict)]
impl UniqueConflictDetector {
    fn is_unique_conflict(&self, err: &DbErr) -> bool {
        sql_err_is_unique(err) || postgres_unique_message(&err.to_string())
    }
}

/// MySQL message patterns only (`Duplicate entry`, `error 1062`,
/// `duplicate key`).
pub struct MysqlUniqueConflict;

#[cgp_impl(MysqlUniqueConflict)]
impl UniqueConflictDetector {
    fn is_unique_conflict(&self, err: &DbErr) -> bool {
        sql_err_is_unique(err) || mysql_unique_message(&err.to_string())
    }
}

/// Default policy used by [`crate::presets::RepositoryLookupStack`].
///
/// Order of attempts:
///
/// 1. [`DbErr::sql_err`] is `UniqueConstraintViolation` (structured,
///    backend-accurate when sqlx/rusqlite is enabled).
/// 2. [`DbErr::RecordNotInserted`] — SeaORM raises this when an insert
///    inserts no rows, commonly `ON CONFLICT DO NOTHING`. This is a
///    documented step of this policy; applications that never use
///    `ON CONFLICT DO NOTHING` opt out by wiring [`SqlErrUniqueConflict`]
///    or a backend-specific policy instead of this default.
/// 3. [`MessageUniqueConflict`] — conservative multi-backend phrases.
///
/// Replace this on the application preset to opt into a tighter policy.
pub struct StandardUniqueConflict;

#[cgp_impl(StandardUniqueConflict)]
impl UniqueConflictDetector {
    fn is_unique_conflict(&self, err: &DbErr) -> bool {
        is_unique_constraint_violation(err)
    }
}

/// Disjunction of two conflict policies: matches if either `A` or `B` does.
///
/// The [`cgp_provider`] impl **is** the provider. Wire the type directly
/// on `UniqueConflictDetectorComponent` — not
/// `UseDelegate<OrUniqueConflict<...>>`.
///
/// ```
/// use cgp_orm::components::UniqueConflictDetector;
/// use cgp_orm::providers::conflict::{
///     MessageUniqueConflict, OrUniqueConflict, SqlErrUniqueConflict,
/// };
/// use sea_orm::DbErr;
///
/// // In a `cgp_preset!` or `delegate_components!` table:
/// //
/// //     UniqueConflictDetectorComponent:
/// //         OrUniqueConflict<SqlErrUniqueConflict, MessageUniqueConflict>,
///
/// struct Ctx;
/// type Policy = OrUniqueConflict<SqlErrUniqueConflict, MessageUniqueConflict>;
///
/// let conflict = DbErr::Custom("UNIQUE constraint failed: items.name".into());
/// assert!(Policy::is_unique_conflict(&Ctx, &conflict));
///
/// let other = DbErr::Custom("FOREIGN KEY constraint failed".into());
/// assert!(!Policy::is_unique_conflict(&Ctx, &other));
///
/// // This pair does not treat RecordNotInserted as a conflict;
/// // only StandardUniqueConflict includes that step.
/// assert!(!Policy::is_unique_conflict(&Ctx, &DbErr::RecordNotInserted));
/// ```
pub struct OrUniqueConflict<A, B>(PhantomData<(A, B)>);

#[cgp_provider(UniqueConflictDetectorComponent)]
impl<Context, A, B> UniqueConflictDetector<Context> for OrUniqueConflict<A, B>
where
    A: UniqueConflictDetector<Context>,
    B: UniqueConflictDetector<Context>,
{
    fn is_unique_conflict(context: &Context, err: &DbErr) -> bool {
        A::is_unique_conflict(context, err) || B::is_unique_conflict(context, err)
    }
}

/// Whether `err` represents a unique-constraint (or primary-key) violation
/// under the [`StandardUniqueConflict`] algorithm.
///
/// The structured and [`DbErr::RecordNotInserted`] checks short-circuit
/// before the error is rendered, so the sqlx path never formats a string.
/// The message fallback renders the error once and scans that one string.
///
/// Prefer wiring a [`CanDetectUniqueConflict`][crate::components::CanDetectUniqueConflict]
/// policy on the context over calling this function from application code.
#[must_use]
pub fn is_unique_constraint_violation(err: &DbErr) -> bool {
    sql_err_is_unique(err)
        || matches!(err, DbErr::RecordNotInserted)
        || message_is_unique(&err.to_string())
}

/// `true` when SeaORM's structured classifier reports a unique violation.
#[must_use]
pub fn sql_err_is_unique(err: &DbErr) -> bool {
    matches!(err.sql_err(), Some(SqlErr::UniqueConstraintViolation(_)))
}

/// Multi-backend message fallback used by [`MessageUniqueConflict`].
///
/// Callers render the error once (`err.to_string()`) and pass that single
/// string here. Every backend matcher scans it with an ASCII
/// case-insensitive substring check, so the fallback performs no further
/// allocation (no per-backend lowercased copy).
#[must_use]
pub fn message_is_unique(message: &str) -> bool {
    postgres_unique_message(message)
        || mysql_unique_message(message)
        || sqlite_unique_message(message)
}

/// PostgreSQL unique-violation phrases.
#[must_use]
pub fn postgres_unique_message(message: &str) -> bool {
    contains_any_ignore_ascii_case(
        message,
        &["23505", "unique violation", "duplicate key value"],
    )
}

/// MySQL unique-violation phrases.
#[must_use]
pub fn mysql_unique_message(message: &str) -> bool {
    contains_any_ignore_ascii_case(
        message,
        &[
            "duplicate entry",
            "duplicate key",
            "error 1062",
            "errno 1062",
            "error code 1062",
            "(1062)",
        ],
    )
}

/// SQLite unique- and primary-key-violation phrases.
///
/// Matches the documented phrases (`unique constraint failed`,
/// `primary key constraint failed`, `SQLITE_CONSTRAINT_UNIQUE`,
/// `SQLITE_CONSTRAINT_PRIMARYKEY`). The shorter token `unique constraint`
/// is not matched on its own: it is a false-positive risk, and
/// `unique constraint failed` already covers SQLite's documented message.
/// The bare string `constraint failed` is also rejected, because it
/// appears in foreign-key and check-constraint errors.
#[must_use]
pub fn sqlite_unique_message(message: &str) -> bool {
    contains_any_ignore_ascii_case(
        message,
        &[
            "unique constraint failed",
            "primary key constraint failed",
            "sqlite_constraint_unique",
            "sqlite_constraint_primarykey",
        ],
    )
}

/// Whether `haystack` contains any of `needles`, ignoring ASCII case.
fn contains_any_ignore_ascii_case(haystack: &str, needles: &[&str]) -> bool {
    needles
        .iter()
        .any(|needle| contains_ignore_ascii_case(haystack, needle))
}

/// Whether `haystack` contains `needle`, ignoring ASCII case.
///
/// Compares byte windows in place, so it allocates nothing. Non-ASCII
/// bytes compare exactly, which is safe for UTF-8: an ASCII needle can
/// only match ASCII bytes.
fn contains_ignore_ascii_case(haystack: &str, needle: &str) -> bool {
    let needle = needle.as_bytes();
    needle.is_empty()
        || haystack
            .as_bytes()
            .windows(needle.len())
            .any(|window| window.eq_ignore_ascii_case(needle))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sqlite_unique_phrases() {
        assert!(sqlite_unique_message(
            "error returned from database: UNIQUE constraint failed: items.name"
        ));
        assert!(sqlite_unique_message("SQLITE_CONSTRAINT_UNIQUE"));
        assert!(sqlite_unique_message("SQLITE_CONSTRAINT_PRIMARYKEY"));
        assert!(sqlite_unique_message(
            "PRIMARY KEY constraint failed: items.id"
        ));
    }

    #[test]
    fn sqlite_does_not_match_fk_or_check() {
        assert!(!sqlite_unique_message("FOREIGN KEY constraint failed"));
        assert!(!sqlite_unique_message(
            "CHECK constraint failed: qty_positive"
        ));
        assert!(!sqlite_unique_message("constraint failed"));
        assert!(!message_is_unique("FOREIGN KEY constraint failed"));
        assert!(!message_is_unique("CHECK constraint failed: qty"));
    }

    #[test]
    fn postgres_phrases() {
        assert!(postgres_unique_message(
            "error: duplicate key value violates unique constraint \"items_name_key\""
        ));
        assert!(postgres_unique_message(
            "ERROR: unique violation (SQLSTATE 23505)"
        ));
        assert!(!postgres_unique_message(
            "ERROR: foreign key violation (SQLSTATE 23503)"
        ));
        assert!(!postgres_unique_message("FOREIGN KEY constraint failed"));
    }

    #[test]
    fn mysql_phrases() {
        assert!(mysql_unique_message(
            "Duplicate entry 'alpha' for key 'items.name'"
        ));
        assert!(mysql_unique_message("ERROR 1062 (23000): Duplicate entry"));
        assert!(mysql_unique_message("errno 1062"));
        assert!(!mysql_unique_message(
            "ERROR 1452 (23000): Cannot add or update a child row"
        ));
    }

    #[test]
    fn standard_recognizes_record_not_inserted() {
        assert!(is_unique_constraint_violation(&DbErr::RecordNotInserted));
        assert!(!is_unique_constraint_violation(&DbErr::RecordNotUpdated));
        assert!(!is_unique_constraint_violation(&DbErr::RecordNotFound(
            "x".into()
        )));
    }

    #[test]
    fn standard_uses_message_fallback() {
        let err = DbErr::Custom("UNIQUE constraint failed: items.name".into());
        assert!(is_unique_constraint_violation(&err));
        let err = DbErr::Custom("CHECK constraint failed: qty".into());
        assert!(!is_unique_constraint_violation(&err));
    }

    #[test]
    fn sqlite_does_not_match_bare_unique_constraint() {
        assert!(!sqlite_unique_message("unique constraint"));
        assert!(!sqlite_unique_message(
            "a unique constraint exists on items.name"
        ));
        // The documented phrase still matches.
        assert!(sqlite_unique_message(
            "UNIQUE constraint failed: items.name"
        ));
        assert!(!message_is_unique("unique constraint"));
    }

    #[test]
    fn ascii_case_insensitive_substring() {
        assert!(contains_ignore_ascii_case(
            "UNIQUE Constraint FAILED",
            "unique constraint failed"
        ));
        assert!(contains_ignore_ascii_case("abc", ""));
        assert!(!contains_ignore_ascii_case("ab", "abc"));
        assert!(!contains_ignore_ascii_case("", "a"));
        // Non-ASCII bytes do not disturb matching around them.
        assert!(contains_ignore_ascii_case(
            "ä Duplicate Entry ö",
            "duplicate entry"
        ));
        assert!(!contains_ignore_ascii_case("ÄBC", "äbc"));
    }

    #[test]
    fn matchers_ignore_ascii_case() {
        assert!(postgres_unique_message("DUPLICATE KEY VALUE violates"));
        assert!(mysql_unique_message("duplicate ENTRY 'a' for key"));
        assert!(mysql_unique_message("Error Code 1062"));
        assert!(sqlite_unique_message("sqlite_constraint_unique"));
        assert!(message_is_unique("Unique Violation"));
    }

    #[test]
    fn or_unique_conflict_composes_sql_err_with_message() {
        struct Ctx;
        type Policy = OrUniqueConflict<SqlErrUniqueConflict, MessageUniqueConflict>;

        // Message policy covers a unique phrase when sql_err is absent.
        let message_only = DbErr::Custom("UNIQUE constraint failed: items.name".into());
        assert!(
            Policy::is_unique_conflict(&Ctx, &message_only),
            "OrUniqueConflict should match via the message policy"
        );
        assert!(!SqlErrUniqueConflict::is_unique_conflict(
            &Ctx,
            &message_only
        ));
        assert!(MessageUniqueConflict::is_unique_conflict(
            &Ctx,
            &message_only
        ));

        // Neither half treats RecordNotInserted as a conflict.
        assert!(!Policy::is_unique_conflict(&Ctx, &DbErr::RecordNotInserted));
        assert!(is_unique_constraint_violation(&DbErr::RecordNotInserted));

        // Foreign-key messages are still rejected.
        let fk = DbErr::Custom("FOREIGN KEY constraint failed".into());
        assert!(!Policy::is_unique_conflict(&Ctx, &fk));
    }
}
