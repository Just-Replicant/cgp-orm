use cgp_fork::prelude::*;
use sea_orm::{EntityTrait, IntoActiveModel};

use crate::components::repository::CanPaginateEntities;
use crate::components::service::HasOutputType;
use crate::components::service::{CanEncodeOutput, ListResources, ListResourcesComponent};
use crate::pagination::{Page, PageRequest};
use crate::types::HasEntityType;

/// Paginate models and encode each row.
pub struct ListResourcesProvider;

#[cgp_impl(ListResourcesProvider)]
#[uses(CanPaginateEntities, CanEncodeOutput)]
#[use_type(HasEntityType.Entity, HasErrorType.Error, HasOutputType.Output)]
impl ListResources
where
    Self: Send + Sync,
    Error: Send,
    Output: Send,
    <Entity as EntityTrait>::Model: IntoActiveModel<<Entity as EntityTrait>::ActiveModel> + Send,
{
    async fn list(&self, request: PageRequest) -> Result<Page<Output>, Error> {
        let page = self.paginate(request).await?;
        page.try_map(|model| self.encode_output(model))
    }
}
