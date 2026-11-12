//! Entity-level query filter applied to list (and identity) lookups.
//!
//! The default provider returns [`Condition::all()`] — no restriction —
//! so every existing context stays valid. Applications supply a filter for
//! tenant isolation, search, or soft delete. The library does **not** grow
//! a `DeletionPolicy`; soft delete is an application filter.

use cgp_fork::prelude::*;
use sea_orm::Condition;

use crate::types::HasEntityType;

/// Build a SeaORM [`Condition`] applied to entity queries.
///
/// [`crate::providers::repository::PaginateEntities`] consults this for
/// both the page query and the `COUNT(*)`.
/// [`crate::providers::repository::FindEntityByIdentity`] also ANDs it
/// onto the identity condition, which stays a no-op under the default
/// empty filter.
///
/// Soft delete (and tenant isolation, search, and similar restrictions)
/// is handled by the application supplying this filter, not by a library
/// deletion policy.
#[cgp_component(EntityFilterBuilder)]
#[use_type(HasEntityType.Entity, HasErrorType.Error)]
pub trait CanBuildEntityFilter {
    /// Return a condition ANDed onto entity queries.
    ///
    /// [`Condition::all()`] is a no-op (matches every row). Return
    /// `Condition::all().add(...)` to restrict, for example, to a
    /// tenant or to rows that have not been soft-deleted.
    fn entity_filter(&self) -> Result<Condition, Error>;
}
