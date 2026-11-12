use cgp_fork::prelude::*;
use sea_orm::{EntityTrait, IntoActiveModel};

use crate::components::repository::CanUpdateEntityById;
use crate::components::service::{
    CanApplyUpdateInput, CanEncodeOutput, HasOutputType, HasUpdateInputType, UpdateResource,
    UpdateResourceComponent,
};
use crate::types::{HasEntityType, PrimaryKeyOf};

/// Apply an update input, merge-update by id, and encode.
pub struct UpdateResourceProvider;

#[cgp_impl(UpdateResourceProvider)]
#[uses(CanApplyUpdateInput, CanUpdateEntityById, CanEncodeOutput)]
#[use_type(
    HasEntityType.Entity,
    HasErrorType.Error,
    HasUpdateInputType.UpdateInput,
    HasOutputType.Output
)]
impl UpdateResource
where
    Self: Send + Sync,
    Error: Send,
    UpdateInput: Send,
    Output: Send,
    PrimaryKeyOf<Self>: Send,
    <Entity as EntityTrait>::Model: IntoActiveModel<<Entity as EntityTrait>::ActiveModel> + Send,
{
    async fn update(
        &self,
        id: PrimaryKeyOf<Self>,
        input: UpdateInput,
    ) -> Result<Option<Output>, Error> {
        let active = self.apply_update_input(id, input)?;
        match self.update_by_id(active).await? {
            Some(model) => Ok(Some(self.encode_output(model)?)),
            None => Ok(None),
        }
    }
}
