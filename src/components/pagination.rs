//! Pagination and deterministic-ordering policies.

use cgp_fork::prelude::*;
use sea_orm::Order;

use crate::types::{ColumnOf, HasEntityType};

/// Default and maximum page sizes. The repository never hardcodes a size of 5.
///
/// The crate's default provider uses a default of 20 and a maximum of 100.
#[cgp_component(PaginationPolicy)]
pub trait HasPaginationPolicy {
    /// Size used when [`crate::pagination::PageRequest::size`] is `0`.
    fn default_page_size(&self) -> u64;

    /// Upper bound; requested sizes above this are clamped.
    fn max_page_size(&self) -> u64;
}

/// Required deterministic ordering for pagination (and a prerequisite for
/// a future cursor/keyset API).
///
/// An empty ordering is an error ([`crate::error::ErrMissingOrdering`]):
/// unordered `LIMIT/OFFSET` is not stable.
#[cgp_component(EntityOrdering)]
#[use_type(HasEntityType.Entity, HasErrorType.Error)]
pub trait CanBuildEntityOrder {
    /// Ordered list of `(column, direction)` pairs. Must not be empty.
    ///
    /// The default [`crate::providers::PrimaryKeyOrdering`] uses every
    /// primary-key column in declaration order, ascending.
    fn entity_order(&self) -> Result<Vec<(ColumnOf<Self>, Order)>, Error>;
}
