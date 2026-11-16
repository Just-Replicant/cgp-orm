use cgp_fork::prelude::*;
use sea_orm::{ActiveModelTrait, EntityTrait, IntoActiveModel, TransactionTrait};

use crate::accessors::HasDbConn;
use crate::components::conflict::CanDetectUniqueConflict;
use crate::components::filter::CanBuildEntityFilter;
use crate::components::identity::CanBuildIdentityCondition;
use crate::components::repository::{
    CanFindEntityByIdentity, EntityGetOrCreator, EntityGetOrCreatorComponent,
};
use crate::components::transaction::CanPrepareEntityWrite;
use crate::error::ErrConflictFetchFailed;
use crate::providers::repository::find_by_identity::find_one_by_identity;
use crate::transaction::conclude_transaction;
use crate::types::{ActiveModelOf, HasEntityType, InsertOutcome, ModelOf};

/// Return the row for this identity, inserting only when it is absent.
///
/// The existing row is loaded first, by reference: the common "row already
/// exists" path never clones the active model. A repeat call does not
/// insert, so it does not run the write hook and does not consume an
/// autoincrement value.
///
/// On a miss, the identity [`Condition`](sea_orm::Condition) is built
/// (copying only the identity values) before the insert consumes the
/// model, so the model is never cloned on either path. This still relies
/// on a **database unique constraint**: if a concurrent insert wins, the
/// insert is rolled back (undoing the write hook) and the winner is loaded
/// on a clean connection with that kept condition ANDed with the entity
/// filter (the same query as [`FindEntityByIdentity`](crate::providers::repository::FindEntityByIdentity)). PostgreSQL
/// aborts a transaction after a unique violation, so that fetch must not
/// use the failed transaction. The preceding select does not by itself
/// prevent concurrent duplicates.
pub struct GetOrCreateEntity;

#[cgp_impl(GetOrCreateEntity)]
#[uses(
    HasDbConn,
    CanPrepareEntityWrite,
    CanFindEntityByIdentity,
    CanBuildIdentityCondition,
    CanBuildEntityFilter,
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
    <Entity as EntityTrait>::ActiveModel: ActiveModelTrait<Entity = Entity> + Send + Sync,
{
    async fn get_or_create(
        &self,
        model: ActiveModelOf<Self>,
    ) -> Result<(ModelOf<Self>, InsertOutcome), Error> {
        if let Some(existing) = self.find_by_identity(&model).await? {
            return Ok((existing, InsertOutcome::Existing));
        }

        // `insert` below takes the model by value. Keep only the identity
        // values (an owned `Condition`) for a possible conflict refetch
        // instead of cloning the whole row.
        let identity = self.identity_condition(&model)?;

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
                let existing = find_one_by_identity::<Entity, _>(
                    self.db_conn(),
                    self.entity_filter()?,
                    identity,
                )
                .await
                .map_err(Self::raise_error)?;
                match existing {
                    Some(existing) => Ok((existing, InsertOutcome::Existing)),
                    None => Err(Self::raise_error(ErrConflictFetchFailed)),
                }
            }
            Err(error) => conclude_transaction(txn, Err(error), Self::raise_error).await,
        }
    }
}
