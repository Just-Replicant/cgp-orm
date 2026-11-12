use cgp_fork::prelude::*;

use crate::components::repository::CanDeleteEntityById;
use crate::components::service::{DeleteResource, DeleteResourceComponent};
use crate::types::{HasEntityType, PrimaryKeyOf};

/// Delete a resource by its complete primary key.
pub struct DeleteResourceProvider;

#[cgp_impl(DeleteResourceProvider)]
#[uses(CanDeleteEntityById)]
#[use_type(HasEntityType.Entity, HasErrorType.Error)]
impl DeleteResource
where
    Self: Send + Sync,
    Error: Send,
    PrimaryKeyOf<Self>: Send,
{
    async fn delete(&self, id: PrimaryKeyOf<Self>) -> Result<bool, Error> {
        self.delete_by_id(id).await
    }
}
