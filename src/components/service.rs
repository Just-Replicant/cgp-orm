//! Generic resource-service layer.
//!
//! Input and output types are associated types chosen by the context, not a
//! fixed DTO. A consumer can map all three to one shared type, or use
//! distinct request and response types.
//!
//! The repository layer is deliberately unaware of these types.

use cgp_fork::prelude::*;

use crate::pagination::{Page, PageRequest};
use crate::types::{ActiveModelOf, HasEntityType, InsertOutcome, ModelOf, PrimaryKeyOf};

/// The payload accepted by [`CanCreateResource`].
#[cgp_type]
pub trait HasCreateInputType {
    /// Application input type for create.
    type CreateInput;
}

/// The payload accepted by [`CanUpdateResource`].
#[cgp_type]
pub trait HasUpdateInputType {
    /// Application input type for update.
    type UpdateInput;
}

/// The payload returned by the resource service.
#[cgp_type]
pub trait HasOutputType {
    /// Application output type.
    type Output;
}

/// Decode a create input into a SeaORM active model (primary key usually `NotSet`).
#[cgp_component(DecodeCreateInput)]
#[use_type(
    HasEntityType.Entity,
    HasErrorType.Error,
    HasCreateInputType.CreateInput
)]
pub trait CanDecodeCreateInput {
    /// Convert `input` into an insertable active model.
    fn decode_create_input(&self, input: CreateInput) -> Result<ActiveModelOf<Self>, Error>;
}

/// Apply an update input, binding it to a complete primary key.
#[cgp_component(ApplyUpdateInput)]
#[use_type(
    HasEntityType.Entity,
    HasErrorType.Error,
    HasUpdateInputType.UpdateInput
)]
pub trait CanApplyUpdateInput {
    /// Convert `input` into an active model whose primary-key columns equal `id`.
    fn apply_update_input(
        &self,
        id: PrimaryKeyOf<Self>,
        input: UpdateInput,
    ) -> Result<ActiveModelOf<Self>, Error>;
}

/// Encode a stored model into the context's output type.
#[cgp_component(EncodeOutput)]
#[use_type(HasEntityType.Entity, HasErrorType.Error, HasOutputType.Output)]
pub trait CanEncodeOutput {
    /// Convert `model` into the application output type.
    fn encode_output(&self, model: ModelOf<Self>) -> Result<Output, Error>;
}

/// Create a resource or return the existing one.
///
/// Decodes `input`, delegates to [`crate::components::CanGetOrCreateEntity::get_or_create`],
/// and encodes the resulting model.
#[cgp_component(CreateResource)]
#[cgp_async(Send)]
#[use_type(HasErrorType.Error, HasCreateInputType.CreateInput, HasOutputType.Output)]
pub trait CanCreateResource {
    /// Decode, get-or-create at the repository layer, and encode.
    async fn create(&self, input: CreateInput) -> Result<(Output, InsertOutcome), Error>;
}

/// Update a resource by primary key.
#[cgp_component(UpdateResource)]
#[cgp_async(Send)]
#[use_type(
    HasEntityType.Entity,
    HasErrorType.Error,
    HasUpdateInputType.UpdateInput,
    HasOutputType.Output
)]
pub trait CanUpdateResource {
    /// Apply `input` to the row `id`. `Ok(None)` if the row does not exist.
    async fn update(
        &self,
        id: PrimaryKeyOf<Self>,
        input: UpdateInput,
    ) -> Result<Option<Output>, Error>;
}

/// Fetch a resource by primary key and encode it.
#[cgp_component(GetResource)]
#[cgp_async(Send)]
#[use_type(HasEntityType.Entity, HasErrorType.Error, HasOutputType.Output)]
pub trait CanGetResource {
    /// `Ok(None)` if the row does not exist.
    async fn get(&self, id: PrimaryKeyOf<Self>) -> Result<Option<Output>, Error>;
}

/// List resources with policy-clamped pagination.
#[cgp_component(ListResources)]
#[cgp_async(Send)]
#[use_type(HasErrorType.Error, HasOutputType.Output)]
pub trait CanListResources {
    /// Paginate and encode each row.
    async fn list(&self, request: PageRequest) -> Result<Page<Output>, Error>;
}

/// Delete a resource by primary key.
///
/// Included so consumers have a complete service surface.
/// Returns `true` if a row was deleted.
#[cgp_component(DeleteResource)]
#[cgp_async(Send)]
#[use_type(HasEntityType.Entity, HasErrorType.Error)]
pub trait CanDeleteResource {
    /// Delete the row `id`.
    async fn delete(&self, id: PrimaryKeyOf<Self>) -> Result<bool, Error>;
}
