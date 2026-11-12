//! Default entity-filter policy: no restriction.
//!
//! Applications override [`crate::components::EntityFilterBuilderComponent`]
//! with a tenant, search, or soft-delete condition. The library does not
//! provide a `DeletionPolicy`; soft delete is that application filter.

use cgp_fork::prelude::*;
use sea_orm::Condition;

use crate::components::filter::{EntityFilterBuilder, EntityFilterBuilderComponent};
use crate::types::HasEntityType;

/// Default filter: [`Condition::all()`], which matches every row.
///
/// Wired by [`crate::presets::RepositoryLookupStack`]. Existing contexts
/// that use the stack stay valid without any new wiring.
pub struct NoEntityFilter;

#[cgp_impl(NoEntityFilter)]
#[use_type(HasEntityType.Entity, HasErrorType.Error)]
impl EntityFilterBuilder {
    fn entity_filter(&self) -> Result<Condition, Error> {
        Ok(Condition::all())
    }
}
