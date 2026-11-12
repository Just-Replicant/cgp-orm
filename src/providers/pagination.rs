//! Default pagination (20 / 100) and primary-key ordering policies.

use cgp_fork::prelude::*;
use sea_orm::entity::{Iterable, PrimaryKeyToColumn};
use sea_orm::{EntityTrait, Order};

use crate::components::pagination::{
    EntityOrdering, EntityOrderingComponent, PaginationPolicy, PaginationPolicyComponent,
};
use crate::error::ErrMissingOrdering;
use crate::types::{ColumnOf, HasEntityType};

/// Pagination policy parameterized by a default and a maximum size.
///
/// Neither default is 5. Use this in tests or in a context that wants
/// limits other than [`DefaultPaginationPolicy`].
pub struct ConstPaginationPolicy<const DEFAULT: u64, const MAX: u64>;

#[cgp_impl(ConstPaginationPolicy<DEFAULT, MAX>)]
impl<const DEFAULT: u64, const MAX: u64> PaginationPolicy {
    fn default_page_size(&self) -> u64 {
        DEFAULT
    }

    fn max_page_size(&self) -> u64 {
        MAX
    }
}

/// Crate-wide default: page size 20, maximum 100. Never 5.
pub struct DefaultPaginationPolicy;

#[cgp_impl(DefaultPaginationPolicy)]
impl PaginationPolicy {
    fn default_page_size(&self) -> u64 {
        20
    }

    fn max_page_size(&self) -> u64 {
        100
    }
}

/// Order by every primary-key column, ascending, in declaration order.
///
/// This is always deterministic for a well-defined entity. An entity with
/// no primary-key columns raises [`ErrMissingOrdering`].
pub struct PrimaryKeyOrdering;

#[cgp_impl(PrimaryKeyOrdering)]
#[uses(CanRaiseError<ErrMissingOrdering>)]
#[use_type(HasEntityType.Entity, HasErrorType.Error)]
impl EntityOrdering
where
    <Entity as EntityTrait>::PrimaryKey:
        PrimaryKeyToColumn<Column = <Entity as EntityTrait>::Column> + Iterable,
{
    fn entity_order(&self) -> Result<Vec<(ColumnOf<Self>, Order)>, Error> {
        let order: Vec<_> = <Entity as EntityTrait>::PrimaryKey::iter()
            .map(|pk| (pk.into_column(), Order::Asc))
            .collect();
        if order.is_empty() {
            Err(Self::raise_error(ErrMissingOrdering))
        } else {
            Ok(order)
        }
    }
}
