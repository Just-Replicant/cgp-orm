use cgp_fork::prelude::*;
use sea_orm::{EntityTrait, PaginatorTrait, QueryOrder};

use crate::accessors::HasDbConn;
use crate::components::pagination::{CanBuildEntityOrder, HasPaginationPolicy};
use crate::components::repository::{EntityPaginator, EntityPaginatorComponent};
use crate::error::ErrMissingOrdering;
use crate::pagination::{Page, PageRequest};
use crate::types::{HasEntityType, ModelOf};

/// Paginate with a required deterministic order and a configurable size policy.
///
/// The page size is never hardcoded to 5. The default provider uses
/// [`crate::providers::DefaultPaginationPolicy`] (20 / 100) and
/// [`crate::providers::PrimaryKeyOrdering`].
pub struct PaginateEntities;

#[cgp_impl(PaginateEntities)]
#[uses(
    HasDbConn,
    HasPaginationPolicy,
    CanBuildEntityOrder,
    CanRaiseError<sea_orm::DbErr>,
    CanRaiseError<ErrMissingOrdering>
)]
#[use_type(HasEntityType.Entity, HasErrorType.Error)]
impl EntityPaginator
where
    Self: Send + Sync,
    Error: Send,
    Entity: Send,
    <Entity as EntityTrait>::Model: Send + Sync,
{
    async fn paginate(&self, request: PageRequest) -> Result<Page<ModelOf<Self>>, Error> {
        let resolved = request.resolve(self.default_page_size(), self.max_page_size());
        let orderings = self.entity_order()?;
        if orderings.is_empty() {
            return Err(Self::raise_error(ErrMissingOrdering));
        }

        let mut select = Entity::find();
        for (column, direction) in orderings {
            select = select.order_by(column, direction);
        }

        let paginator = PaginatorTrait::paginate(select, self.db_conn(), resolved.size);
        let total = paginator.num_items().await.map_err(Self::raise_error)?;
        let items = paginator
            .fetch_page(resolved.zero_based_page())
            .await
            .map_err(Self::raise_error)?;

        Ok(Page {
            items,
            page: resolved.page,
            size: resolved.size,
            total,
        })
    }
}
