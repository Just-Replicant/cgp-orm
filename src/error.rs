//! Typed, transport-independent repository errors.
//!
//! These are **source** errors. A context raises them into its own error type
//! through CGP's `CanRaiseError`. Application-specific sources and transport
//! error types do not belong here.

use core::fmt::Debug;

use thiserror::Error;

/// An identity policy produced no columns or no usable condition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
#[error("identity condition is empty; the identity policy must constrain at least one column")]
pub struct ErrIdentityEmpty;

/// A column required by the identity policy is `NotSet` on the active model.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
#[error("identity field {column:?} is not set")]
pub struct ErrIdentityFieldUnset<Column> {
    /// The unset column.
    pub column: Column,
}

/// The entity has no primary-key columns, so an id cannot be read or written.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("entity {entity_name} has no primary-key columns")]
pub struct ErrMissingPrimaryKey {
    /// The entity type name (`core::any::type_name`).
    pub entity_name: &'static str,
}

impl ErrMissingPrimaryKey {
    /// Build an error that names entity `E`.
    #[must_use]
    pub fn of<E>() -> Self {
        Self {
            entity_name: core::any::type_name::<E>(),
        }
    }
}

/// A required primary-key column is `NotSet` on the active model.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
#[error("primary-key field {column:?} is not set")]
pub struct ErrPrimaryKeyUnset<Column> {
    /// The unset primary-key column.
    pub column: Column,
}

/// The entity's primary key has an arity this crate cannot encode.
///
/// Raised when a key has **zero** columns. SeaORM 2 encodes arity 1–3 as
/// dedicated [`sea_orm::ValueTuple`] variants and arity 4+ as
/// `ValueTuple::Many`, all of which are supported.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
#[error("unsupported primary-key arity: {arity}")]
pub struct ErrUnsupportedKeyArity {
    /// Number of primary-key columns encountered.
    pub arity: usize,
}

/// Pagination was requested without a deterministic ordering policy.
///
/// Offset pagination is not stable unless the query is totally ordered.
/// Contexts must wire [`crate::components::CanBuildEntityOrder`] (the default
/// [`crate::providers::PrimaryKeyOrdering`] is sufficient).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
#[error("pagination requires a deterministic ordering policy, but none was provided")]
pub struct ErrMissingOrdering;

/// `insert_or_get` observed a unique-constraint conflict, then could not
/// fetch the existing row by identity.
///
/// This usually means the identity policy does not match the unique
/// constraint that fired, or the conflicting row was deleted in between.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
#[error(
    "unique constraint conflict was observed, but the existing row could not be fetched by identity"
)]
pub struct ErrConflictFetchFailed;

/// A resource or entity looked up by primary key was not found.
///
/// The generic layer does not assume an `i32` identifier or an entity
/// route name. Application error maps may wrap this with more context.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("entity not found")]
pub struct ErrEntityNotFound;

/// A unique-constraint violation occurred and the chosen policy is to fail.
///
/// [`crate::components::CanInsertEntity::insert`] raises the original
/// [`sea_orm::DbErr`] by default. This type is available for policies that
/// want a dedicated source error.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
#[error("unique constraint violated")]
pub struct ErrUniqueConflict;
