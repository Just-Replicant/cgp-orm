use cgp_fork::prelude::*;
use sea_orm::{EntityTrait, IntoActiveModel};

use crate::components::repository::CanGetOrCreateEntity;
use crate::components::service::{
    CanDecodeCreateInput, CanEncodeOutput, GetOrCreateResource, GetOrCreateResourceComponent,
    HasCreateInputType, HasOutputType,
};
use crate::types::{HasEntityType, InsertOutcome};

/// Decode, [`get_or_create`](CanGetOrCreateEntity::get_or_create), and encode.
pub struct GetOrCreateResourceProvider;

#[cgp_impl(GetOrCreateResourceProvider)]
#[uses(CanDecodeCreateInput, CanGetOrCreateEntity, CanEncodeOutput)]
#[use_type(HasEntityType.Entity, HasErrorType.Error, HasCreateInputType.CreateInput, HasOutputType.Output)]
impl GetOrCreateResource
where
    Self: Send + Sync,
    Error: Send,
    CreateInput: Send,
    Output: Send,
    <Entity as EntityTrait>::Model: IntoActiveModel<<Entity as EntityTrait>::ActiveModel> + Send,
{
    async fn get_or_create(&self, input: CreateInput) -> Result<(Output, InsertOutcome), Error> {
        let active = self.decode_create_input(input)?;
        let (model, outcome) = CanGetOrCreateEntity::get_or_create(self, active).await?;
        Ok((self.encode_output(model)?, outcome))
    }
}
