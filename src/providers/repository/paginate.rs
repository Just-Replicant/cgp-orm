use cgp_fork::prelude::*;
use sea_orm::{EntityTrait, PaginatorTrait, QueryFilter, QueryOrder};

use crate::accessors::HasDbConn;
use crate::components::filter::CanBuildEntityFilter;
use crate::components::pagination::{CanBuildEntityOrder, HasPaginationPolicy};
use crate::components::repository::{EntityPaginator, EntityPaginatorComponent};
use crate::error::ErrMissingOrdering;
use crate::pagination::{Page, PageRequest};
use crate::types::{HasEntityType, ModelOf};

/// Paginate with a required deterministic order, a configurable size policy,
/// and the context's entity filter.
///
/// The page size is never hardcoded to 5. The default provider uses
/// [`crate::providers::DefaultPaginationPolicy`] (20 / 100) and
/// [`crate::providers::PrimaryKeyOrdering`].
///
/// The WHERE clause comes from [`CanBuildEntityFilter`]. The default
/// [`crate::providers::NoEntityFilter`] returns [`sea_orm::Condition::all()`],
/// so existing contexts list every row. Both the page query and the
/// `COUNT(*)` use this filter. When [`PageRequest::include_total`] is
/// `false`, the count is skipped and [`Page::total`] is `None`.
///
/// Soft delete is an application filter on this component, not a library
/// `DeletionPolicy`.
pub struct PaginateEntities;

#[cgp_impl(PaginateEntities)]
#[uses(
    HasDbConn,
    HasPaginationPolicy,
    CanBuildEntityOrder,
    CanBuildEntityFilter,
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

        let mut select = Entity::find().filter(self.entity_filter()?);
        for (column, direction) in orderings {
            select = select.order_by(column, direction);
        }

        let paginator = PaginatorTrait::paginate(select, self.db_conn(), resolved.size);
        let total = if resolved.include_total {
            Some(paginator.num_items().await.map_err(Self::raise_error)?)
        } else {
            None
        };
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
