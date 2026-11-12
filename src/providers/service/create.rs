use cgp_fork::prelude::*;
use sea_orm::{EntityTrait, IntoActiveModel};

use crate::components::repository::CanInsertEntity;
use crate::components::service::{
    CanDecodeCreateInput, CanEncodeOutput, CreateResource, CreateResourceComponent,
    HasCreateInputType, HasOutputType,
};
use crate::types::HasEntityType;

/// Decode, [`insert`](CanInsertEntity::insert), and encode. Conflicts fail.
pub struct CreateResourceProvider;

#[cgp_impl(CreateResourceProvider)]
#[uses(CanDecodeCreateInput, CanInsertEntity, CanEncodeOutput)]
#[use_type(HasEntityType.Entity, HasErrorType.Error, HasCreateInputType.CreateInput, HasOutputType.Output)]
impl CreateResource
where
    Self: Send + Sync,
    Error: Send,
    CreateInput: Send,
    Output: Send,
    <Entity as EntityTrait>::Model: IntoActiveModel<<Entity as EntityTrait>::ActiveModel> + Send,
{
    async fn create(&self, input: CreateInput) -> Result<Output, Error> {
        let active = self.decode_create_input(input)?;
        let model = self.insert(active).await?;
        self.encode_output(model)
    }
}
