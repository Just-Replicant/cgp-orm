//! Context accessors: the database connection and unique-column lists.
//!
//! [`HasDbConn`] is auto-derived from a `db_conn` field via `#[cgp_auto_getter]`.
//! [`HasUniqueColumns`] is a policy getter used by
//! [`crate::providers::UniqueColumnsIdentity`]; it is **not** the identity API
//! itself. The identity API is
//! [`crate::components::CanBuildIdentityCondition`].

use cgp_fork::prelude::*;
use sea_orm::DatabaseConnection;

use crate::types::{ColumnOf, HasEntityType};

/// Accessor for the SeaORM database connection.
///
/// A context with a `db_conn: DatabaseConnection` field gets this for free.
#[cgp_auto_getter]
pub trait HasDbConn {
    /// Borrow the connection used by repository providers.
    fn db_conn(&self) -> &DatabaseConnection;
}

/// Getter for the columns that form a simple unique identity.
///
/// [`crate::providers::UniqueColumnsIdentity`] turns this list into a
/// [`sea_orm::Condition`]. Contexts with a more complex identity skip this
/// getter and implement [`crate::components::CanBuildIdentityCondition`]
/// directly.
#[cgp_getter]
#[use_type(HasEntityType.Entity)]
pub trait HasUniqueColumns {
    /// Columns that together identify a unique row (must match a database
    /// unique constraint used by `get_or_create`).
    fn unique_columns(&self) -> Vec<ColumnOf<Self>>;
}
