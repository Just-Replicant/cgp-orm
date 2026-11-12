//! Repository-layer consumer traits.
//!
//! These operate on SeaORM [`Model`](sea_orm::EntityTrait::Model) values and
//! [`PrimaryKeyOf`](crate::types::PrimaryKeyOf). They do not know about DTOs.

use cgp_fork::prelude::*;

use crate::pagination::{Page, PageRequest};
use crate::types::{
    ActiveModelOf, HasEntityType, InsertOutcome, ModelOf, PrimaryKeyOf, UpsertOutcome,
};

/// Look up a row by its complete SeaORM primary-key value.
#[cgp_component(EntityByIdFinder)]
#[cgp_async(Send)]
#[use_type(HasEntityType.Entity, HasErrorType.Error)]
pub trait CanFindEntityById {
    /// Fetch the model with this primary key, or `Ok(None)` if it does not exist.
    async fn find_by_id(&self, id: PrimaryKeyOf<Self>) -> Result<Option<ModelOf<Self>>, Error>;
}

/// Insert a new row. Unique-constraint violations are returned as errors.
#[cgp_component(EntityInserter)]
#[cgp_async(Send)]
#[use_type(HasEntityType.Entity, HasErrorType.Error)]
pub trait CanInsertEntity {
    /// Insert `model` and return the stored row (with generated keys filled in).
    async fn insert(&self, model: ActiveModelOf<Self>) -> Result<ModelOf<Self>, Error>;
}

/// Merge-update a row identified by the primary key carried on `model`.
///
/// All primary-key columns on `model` must be set. They are used for lookup
/// and are never overwritten by the merge.
#[cgp_component(EntityByIdUpdater)]
#[cgp_async(Send)]
#[use_type(HasEntityType.Entity, HasErrorType.Error)]
pub trait CanUpdateEntityById {
    /// Update the existing row, or `Ok(None)` if the primary key was not found.
    async fn update_by_id(
        &self,
        model: ActiveModelOf<Self>,
    ) -> Result<Option<ModelOf<Self>>, Error>;
}

/// Delete a row by its complete SeaORM primary-key value.
#[cgp_component(EntityByIdDeleter)]
#[cgp_async(Send)]
#[use_type(HasEntityType.Entity, HasErrorType.Error)]
pub trait CanDeleteEntityById {
    /// Return `true` if a row was deleted, `false` if the key was not found.
    async fn delete_by_id(&self, id: PrimaryKeyOf<Self>) -> Result<bool, Error>;
}

/// Look up a row by the context's identity policy (not necessarily the PK).
#[cgp_component(EntityIdentityFinder)]
#[cgp_async(Send)]
#[use_type(HasEntityType.Entity, HasErrorType.Error)]
pub trait CanFindEntityByIdentity {
    /// Fetch at most one row matching the identity of `model`.
    async fn find_by_identity(
        &self,
        model: &ActiveModelOf<Self>,
    ) -> Result<Option<ModelOf<Self>>, Error>;
}

/// Insert a row, or return the existing one when a unique constraint fires.
///
/// This operation **relies on a database unique constraint**. A transaction
/// around find-then-insert is not used and would not prevent concurrent
/// duplicates by itself. On conflict the existing row is fetched by identity.
#[cgp_component(EntityInsertOrGetter)]
#[cgp_async(Send)]
#[use_type(HasEntityType.Entity, HasErrorType.Error)]
pub trait CanInsertOrGetEntity {
    /// Insert `model`, or fetch the conflicting row.
    async fn insert_or_get(
        &self,
        model: ActiveModelOf<Self>,
    ) -> Result<(ModelOf<Self>, InsertOutcome), Error>;
}

/// Insert a row, or merge-update the existing one (identity + unique constraint).
#[cgp_component(EntityUpserter)]
#[cgp_async(Send)]
#[use_type(HasEntityType.Entity, HasErrorType.Error)]
pub trait CanUpsertEntity {
    /// Insert or update `model` and report which path was taken.
    async fn upsert(
        &self,
        model: ActiveModelOf<Self>,
    ) -> Result<(ModelOf<Self>, UpsertOutcome), Error>;
}

/// Fetch a deterministic page of rows.
///
/// The page size is taken from [`PageRequest`] and clamped by
/// [`crate::components::HasPaginationPolicy`]. An ordering policy is
/// required; see [`crate::components::CanBuildEntityOrder`].
#[cgp_component(EntityPaginator)]
#[cgp_async(Send)]
#[use_type(HasEntityType.Entity, HasErrorType.Error)]
pub trait CanPaginateEntities {
    /// Return one page of models in the policy's deterministic order.
    async fn paginate(&self, request: PageRequest) -> Result<Page<ModelOf<Self>>, Error>;
}
