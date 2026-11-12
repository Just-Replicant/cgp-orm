use std::fmt::Debug;

use cgp_fork::prelude::*;
use sea_orm::entity::{ColumnTrait, IdenStatic, PrimaryKeyToColumn};
use sea_orm::{
    ActiveModelTrait, EntityTrait, IntoActiveModel, Iterable, QueryFilter, TransactionTrait,
};

use crate::accessors::HasDbConn;
use crate::components::conflict::CanDetectUniqueConflict;
use crate::components::identity::CanBuildIdentityCondition;
use crate::components::repository::{EntityUpserter, EntityUpserterComponent};
use crate::components::transaction::CanPrepareEntityWrite;
use crate::error::{ErrConflictFetchFailed, ErrMissingPrimaryKey};
use crate::transaction::conclude_transaction;
use crate::types::{ColumnOf, HasEntityType, ModelOf, UpsertOutcome};
use crate::util::{MergeUpdateError, merge_update};

/// Insert a row, or merge-update the existing one.
///
/// The identity policy locates the existing row, and a hit updates it
/// without inserting. If a concurrent insert wins the unique-constraint
/// race, the failed transaction is rolled back and the merge runs on a
/// new one. PostgreSQL aborts a transaction after a unique violation, so
/// the follow-up select cannot use that same transaction. A
/// find-then-insert transaction is not atomic by itself.
pub struct UpsertEntity;

#[cgp_impl(UpsertEntity)]
#[uses(
    HasDbConn,
    CanPrepareEntityWrite,
    CanBuildIdentityCondition,
    CanDetectUniqueConflict,
    CanRaiseError<sea_orm::DbErr>,
    CanRaiseError<ErrMissingPrimaryKey>,
    CanRaiseError<ErrConflictFetchFailed>
)]
#[use_type(HasEntityType.Entity, HasErrorType.Error)]
impl EntityUpserter
where
    Self: Send + Sync,
    Error: Send,
    Entity: Send,
    <Entity as EntityTrait>::Model: IntoActiveModel<<Entity as EntityTrait>::ActiveModel> + Send,
    <Entity as EntityTrait>::ActiveModel: ActiveModelTrait<Entity = Entity> + Send + Sync + Clone,
    ColumnOf<Self>: ColumnTrait + Copy + Debug + IdenStatic,
    <Entity as EntityTrait>::PrimaryKey: PrimaryKeyToColumn<Column = ColumnOf<Self>> + Iterable,
{
    async fn upsert(
        &self,
        model: <Entity as EntityTrait>::ActiveModel,
    ) -> Result<(ModelOf<Self>, UpsertOutcome), Error> {
        let condition = self.identity_condition(&model)?;
        let txn = self.db_conn().begin().await.map_err(Self::raise_error)?;

        let result = async {
            match Entity::find().filter(condition).one(&txn).await {
                Ok(Some(existing)) => {
                    self.prepare_entity_write(&txn, &model).await?;
                    merge_and_update::<Entity, Error, Self>(&txn, existing, model)
                        .await
                        .map(Ok)
                }
                Ok(None) => {
                    self.prepare_entity_write(&txn, &model).await?;
                    match model.clone().insert(&txn).await {
                        Ok(inserted) => Ok(Ok((inserted, UpsertOutcome::Inserted))),
                        Err(db_err) if self.is_unique_conflict(&db_err) => {
                            // The insert lost the race. Hand the model back
                            // so the outer path can roll this transaction
                            // back and merge on a clean connection.
                            Ok(Err(model))
                        }
                        Err(db_err) => Err(Self::raise_error(db_err)),
                    }
                }
                Err(err) => Err(Self::raise_error(err)),
            }
        }
        .await;

        match result {
            Ok(Ok(outcome)) => conclude_transaction(txn, Ok(outcome), Self::raise_error).await,
            Ok(Err(model)) => {
                txn.rollback().await.map_err(Self::raise_error)?;
                let condition = self.identity_condition(&model)?;
                let txn = self.db_conn().begin().await.map_err(Self::raise_error)?;
                let result = async {
                    match Entity::find()
                        .filter(condition)
                        .one(&txn)
                        .await
                        .map_err(Self::raise_error)?
                    {
                        Some(existing) => {
                            self.prepare_entity_write(&txn, &model).await?;
                            merge_and_update::<Entity, Error, Self>(&txn, existing, model).await
                        }
                        None => Err(Self::raise_error(ErrConflictFetchFailed)),
                    }
                }
                .await;
                conclude_transaction(txn, result, Self::raise_error).await
            }
            Err(error) => conclude_transaction(txn, Err(error), Self::raise_error).await,
        }
    }
}

async fn merge_and_update<E, Err, Ctx>(
    txn: &sea_orm::DatabaseTransaction,
    existing: E::Model,
    model: E::ActiveModel,
) -> Result<(E::Model, UpsertOutcome), Err>
where
    E: EntityTrait,
    E::Model: IntoActiveModel<E::ActiveModel> + Send,
    E::ActiveModel: ActiveModelTrait<Entity = E> + Send,
    E::Column: ColumnTrait + Copy + IdenStatic,
    E::PrimaryKey: PrimaryKeyToColumn<Column = E::Column> + Iterable,
    Ctx: CanRaiseError<sea_orm::DbErr, Error = Err>
        + CanRaiseError<ErrMissingPrimaryKey, Error = Err>,
{
    match merge_update::<E>(existing, model) {
        Ok(merged) => merged
            .update(txn)
            .await
            .map(|updated| (updated, UpsertOutcome::Updated))
            .map_err(Ctx::raise_error),
        Err(MergeUpdateError::MissingPrimaryKey(err)) => Err(Ctx::raise_error(err)),
        Err(MergeUpdateError::Db(err)) => Err(Ctx::raise_error(err)),
    }
}
