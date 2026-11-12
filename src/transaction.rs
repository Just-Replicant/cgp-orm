//! Transaction completion that preserves the original failure.
//!
//! The original implementation replaced a failed operation with a rollback
//! error whenever rollback also failed:
//!
//! ```ignore
//! Err(err) => {
//!     txn.rollback().await.map_err(Self::raise_error)?;
//!     Err(err)
//! }
//! ```
//!
//! That `?` drops `err`. This module's helpers never do that: a rollback
//! error is reported alongside the original, or discarded, but the original
//! failure is always the one returned (optionally wrapped).

use sea_orm::{DatabaseConnection, DatabaseTransaction, DbErr, TransactionTrait};

/// How a rollback error is combined with the original operation error.
///
/// The original error is always preserved as the value that is returned;
/// the rollback error is only attached.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RollbackPolicy {
    /// Return the original error and ignore a failed rollback.
    #[default]
    PreserveOriginal,
}

/// Result of concluding a transaction after the inner operation has finished.
///
/// Exposed so tests can inspect both sides without a live database.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransactionConclusion<T, E> {
    /// The operation and the commit both succeeded.
    Committed(T),
    /// The operation succeeded but commit failed. The commit error is `E`.
    CommitFailed(E),
    /// The operation failed. Rollback either succeeded or failed; the
    /// original error is always `operation`. A rollback error, if any, is
    /// stored in `rollback` and does **not** replace `operation`.
    RolledBack {
        /// The original inner-operation error.
        operation: E,
        /// The rollback error, if rollback itself failed.
        rollback: Option<E>,
    },
}

impl<T, E> TransactionConclusion<T, E> {
    /// Collapse into a single `Result`, always preferring the original
    /// operation error over a rollback error.
    pub fn into_result(self) -> Result<T, E> {
        match self {
            Self::Committed(value) => Ok(value),
            Self::CommitFailed(error) => Err(error),
            Self::RolledBack {
                operation,
                rollback: _,
            } => Err(operation),
        }
    }

    /// The original operation error, if any. A rollback-only failure is
    /// not an operation error (commit-failed is a finish error).
    pub fn operation_error(&self) -> Option<&E> {
        match self {
            Self::RolledBack { operation, .. } => Some(operation),
            Self::CommitFailed(_) | Self::Committed(_) => None,
        }
    }

    /// The rollback error, if rollback was attempted and failed.
    pub fn rollback_error(&self) -> Option<&E> {
        match self {
            Self::RolledBack { rollback, .. } => rollback.as_ref(),
            Self::CommitFailed(_) | Self::Committed(_) => None,
        }
    }
}

/// Combine an inner operation result with the result of commit or rollback.
///
/// This is the pure decision procedure used by [`conclude_transaction`].
/// It is unit-tested without a database, including the case where rollback
/// also fails.
///
/// - If `operation` is `Ok`, `finish` is interpreted as the commit result.
/// - If `operation` is `Err`, `finish` is interpreted as the rollback result
///   and never replaces the original error.
#[must_use]
pub fn conclude<T, E>(
    operation: Result<T, E>,
    finish: Result<(), E>,
) -> TransactionConclusion<T, E> {
    match operation {
        Ok(value) => match finish {
            Ok(()) => TransactionConclusion::Committed(value),
            Err(error) => TransactionConclusion::CommitFailed(error),
        },
        Err(operation) => TransactionConclusion::RolledBack {
            operation,
            rollback: finish.err(),
        },
    }
}

/// Commit on success, roll back on failure, and never let a rollback error
/// replace the original failure.
///
/// `raise` converts a SeaORM [`DbErr`] from commit or rollback into `E`.
/// On the failure path the raised rollback error is stored, then dropped
/// from the returned `Result` — the original `result` error is returned.
pub async fn conclude_transaction<T, E>(
    txn: DatabaseTransaction,
    result: Result<T, E>,
    raise: impl Fn(DbErr) -> E,
) -> Result<T, E> {
    match result {
        Ok(value) => {
            txn.commit().await.map_err(raise)?;
            Ok(value)
        }
        Err(error) => {
            // Intentionally do not use `?` on rollback: a rollback error
            // must not replace `error`.
            let _rollback = txn.rollback().await.map_err(&raise);
            let _ = _rollback;
            Err(error)
        }
    }
}

/// Run `op` inside a new transaction on `conn`.
///
/// On success the transaction is committed. On failure it is rolled back
/// and the original error is returned even if rollback also fails.
pub async fn run_in_transaction<T, E, F, Fut>(
    conn: &DatabaseConnection,
    raise: impl Fn(DbErr) -> E,
    op: F,
) -> Result<T, E>
where
    F: FnOnce(&DatabaseTransaction) -> Fut,
    Fut: core::future::Future<Output = Result<T, E>>,
{
    let txn = conn.begin().await.map_err(&raise)?;
    let result = op(&txn).await;
    conclude_transaction(txn, result, raise).await
}

#[cfg(test)]
#[allow(missing_docs)]
mod tests {
    use super::{TransactionConclusion, conclude};

    #[test]
    fn success_and_commit() {
        let conclusion = conclude::<i32, &str>(Ok(7), Ok(()));
        assert_eq!(conclusion, TransactionConclusion::Committed(7));
        assert_eq!(conclusion.into_result(), Ok(7));
    }

    #[test]
    fn success_but_commit_fails() {
        let conclusion = conclude::<i32, &str>(Ok(7), Err("commit failed"));
        assert_eq!(
            conclusion,
            TransactionConclusion::CommitFailed("commit failed")
        );
        assert_eq!(conclusion.into_result(), Err("commit failed"));
    }

    #[test]
    fn failure_and_successful_rollback() {
        let conclusion = conclude::<i32, &str>(Err("hook failed"), Ok(()));
        match &conclusion {
            TransactionConclusion::RolledBack {
                operation,
                rollback,
            } => {
                assert_eq!(*operation, "hook failed");
                assert_eq!(*rollback, None);
            }
            other => panic!("unexpected {other:?}"),
        }
        assert_eq!(conclusion.into_result(), Err("hook failed"));
    }

    #[test]
    fn failure_and_failed_rollback_preserves_original() {
        let conclusion = conclude::<i32, &str>(Err("hook failed"), Err("rollback failed"));
        match &conclusion {
            TransactionConclusion::RolledBack {
                operation,
                rollback,
            } => {
                assert_eq!(*operation, "hook failed");
                assert_eq!(*rollback, Some("rollback failed"));
            }
            other => panic!("unexpected {other:?}"),
        }
        assert_eq!(conclusion.operation_error(), Some(&"hook failed"));
        assert_eq!(conclusion.rollback_error(), Some(&"rollback failed"));
        // The original failure is what the caller observes.
        assert_eq!(conclusion.into_result(), Err("hook failed"));
    }
}
