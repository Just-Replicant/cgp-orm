use cgp_fork::prelude::*;
use sea_orm::{ActiveModelTrait, EntityTrait, IntoActiveModel, TransactionTrait};

use crate::accessors::HasDbConn;
use crate::components::conflict::CanDetectUniqueConflict;
use crate::components::repository::{
    CanFindEntityByIdentity, EntityGetOrCreator, EntityGetOrCreatorComponent,
};
use crate::components::transaction::CanPrepareEntityWrite;
use crate::error::ErrConflictFetchFailed;
use crate::transaction::conclude_transaction;
use crate::types::{ActiveModelOf, HasEntityType, InsertOutcome, ModelOf};

/// Return the row for this identity, inserting only when it is absent.
///
/// The existing row is loaded first. A repeat call does not insert, so it
/// does not run the write hook and does not consume an autoincrement value.
/// This still relies on a **database unique constraint**: if a concurrent
/// insert wins, the insert is rolled back (undoing the write hook) and the
/// winner is loaded on a clean connection. PostgreSQL aborts a transaction
/// after a unique violation, so that fetch must not use the failed
/// transaction. The preceding select does not by itself prevent concurrent
/// duplicates.
pub struct GetOrCreateEntity;

#[cgp_impl(GetOrCreateEntity)]
#[uses(
    HasDbConn,
    CanPrepareEntityWrite,
    CanFindEntityByIdentity,
    CanDetectUniqueConflict,
    CanRaiseError<sea_orm::DbErr>,
    CanRaiseError<ErrConflictFetchFailed>
)]
#[use_type(HasEntityType.Entity, HasErrorType.Error)]
impl EntityGetOrCreator
where
    Self: Send + Sync,
    Error: Send,
    Entity: Send,
    <Entity as EntityTrait>::Model: IntoActiveModel<<Entity as EntityTrait>::ActiveModel> + Send,
    <Entity as EntityTrait>::ActiveModel: ActiveModelTrait<Entity = Entity> + Send + Sync + Clone,
{
    async fn get_or_create(
        &self,
        model: ActiveModelOf<Self>,
    ) -> Result<(ModelOf<Self>, InsertOutcome), Error> {
        let identity_model = model.clone();
        if let Some(existing) = self.find_by_identity(&identity_model).await? {
            return Ok((existing, InsertOutcome::Existing));
        }

        let txn = self.db_conn().begin().await.map_err(Self::raise_error)?;
        let result = async {
            self.prepare_entity_write(&txn, &model).await?;
            match model.insert(&txn).await {
                Ok(inserted) => Ok(Ok((inserted, InsertOutcome::Inserted))),
                Err(db_err) if self.is_unique_conflict(&db_err) => {
                    // Signal "expected conflict" to the outer match so we
                    // roll back (undoing the hook) and fetch by identity
                    // on a clean connection. PostgreSQL aborts a
                    // transaction after a unique violation, so we must
                    // not continue using `txn`.
                    Ok(Err(()))
                }
                Err(db_err) => Err(Self::raise_error(db_err)),
            }
        }
        .await;

        match result {
            Ok(Ok(outcome)) => conclude_transaction(txn, Ok(outcome), Self::raise_error).await,
            Ok(Err(())) => {
                // Expected unique conflict: roll back, then fetch.
                // If rollback fails there is no "original operation error"
                // besides the conflict itself; a failed rollback is then
                // the error we report.
                txn.rollback().await.map_err(Self::raise_error)?;
                match self.find_by_identity(&identity_model).await? {
                    Some(existing) => Ok((existing, InsertOutcome::Existing)),
                    None => Err(Self::raise_error(ErrConflictFetchFailed)),
                }
            }
            Err(error) => conclude_transaction(txn, Err(error), Self::raise_error).await,
        }
    }
}
