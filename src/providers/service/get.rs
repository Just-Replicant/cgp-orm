use cgp_fork::prelude::*;
use sea_orm::{EntityTrait, IntoActiveModel};

use crate::components::repository::CanFindEntityById;
use crate::components::service::HasOutputType;
use crate::components::service::{CanEncodeOutput, GetResource, GetResourceComponent};
use crate::types::{HasEntityType, PrimaryKeyOf};

/// Fetch by id and encode.
pub struct GetResourceProvider;

#[cgp_impl(GetResourceProvider)]
#[uses(CanFindEntityById, CanEncodeOutput)]
#[use_type(HasEntityType.Entity, HasErrorType.Error, HasOutputType.Output)]
impl GetResource
where
    Self: Send + Sync,
    Error: Send,
    Output: Send,
    PrimaryKeyOf<Self>: Send,
    <Entity as EntityTrait>::Model: IntoActiveModel<<Entity as EntityTrait>::ActiveModel> + Send,
{
    async fn get(&self, id: PrimaryKeyOf<Self>) -> Result<Option<Output>, Error> {
        match self.find_by_id(id).await? {
            Some(model) => Ok(Some(self.encode_output(model)?)),
            None => Ok(None),
        }
    }
}
