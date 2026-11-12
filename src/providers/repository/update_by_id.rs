use std::fmt::Debug;

use cgp_fork::prelude::*;
use sea_orm::entity::{ColumnTrait, PrimaryKeyToColumn};
use sea_orm::{ActiveModelTrait, EntityTrait, IntoActiveModel, Iterable, TransactionTrait};

use crate::accessors::HasDbConn;
use crate::components::repository::{EntityByIdUpdater, EntityByIdUpdaterComponent};
use crate::components::transaction::CanPrepareEntityWrite;
use crate::error::{ErrMissingPrimaryKey, ErrPrimaryKeyUnset, ErrUnsupportedKeyArity};
use crate::transaction::conclude_transaction;
use crate::types::{ColumnOf, HasEntityType, ModelOf};
use crate::util::{
    MergeUpdateError, PrimaryKeyReadError, merge_update, primary_key_from_active_model,
};

/// Merge-update a row identified by the complete primary key on `model`.
///
/// Every primary-key column is read for the lookup and excluded from the
/// merge. There is no `primary_key_column()` that returns only the first key.
pub struct UpdateEntityById;

#[cgp_impl(UpdateEntityById)]
#[uses(
    HasDbConn,
    CanPrepareEntityWrite,
    CanRaiseError<sea_orm::DbErr>,
    CanRaiseError<ErrPrimaryKeyUnset<ColumnOf<Self>>>,
    CanRaiseError<ErrMissingPrimaryKey>,
    CanRaiseError<ErrUnsupportedKeyArity>
)]
#[use_type(HasEntityType.Entity, HasErrorType.Error)]
impl EntityByIdUpdater
where
    Self: Send + Sync,
    Error: Send,
    Entity: Send,
    <Entity as EntityTrait>::Model: IntoActiveModel<<Entity as EntityTrait>::ActiveModel> + Send,
    <Entity as EntityTrait>::ActiveModel: ActiveModelTrait<Entity = Entity> + Send + Sync,
    ColumnOf<Self>: ColumnTrait + Copy + Debug,
    <Entity as EntityTrait>::PrimaryKey: PrimaryKeyToColumn<Column = ColumnOf<Self>> + Iterable,
{
    async fn update_by_id(
        &self,
        model: <Entity as EntityTrait>::ActiveModel,
    ) -> Result<Option<ModelOf<Self>>, Error> {
        let id = match primary_key_from_active_model::<Entity>(&model) {
            Ok(id) => id,
            Err(PrimaryKeyReadError::Missing(err)) => return Err(Self::raise_error(err)),
            Err(PrimaryKeyReadError::Unset(err)) => return Err(Self::raise_error(err)),
            Err(PrimaryKeyReadError::Arity(err)) => return Err(Self::raise_error(err)),
        };

        let txn = self.db_conn().begin().await.map_err(Self::raise_error)?;
        let result = async {
            match Entity::find_by_id(id).one(&txn).await {
                Ok(Some(existing)) => {
                    self.prepare_entity_write(&txn, &model).await?;
                    match merge_update::<Entity>(existing, model) {
                        Ok(merged) => merged
                            .update(&txn)
                            .await
                            .map(Some)
                            .map_err(Self::raise_error),
                        Err(MergeUpdateError::MissingPrimaryKey(err)) => {
                            Err(Self::raise_error(err))
                        }
                        Err(MergeUpdateError::Db(err)) => Err(Self::raise_error(err)),
                    }
                }
                Ok(None) => Ok(None),
                Err(err) => Err(Self::raise_error(err)),
            }
        }
        .await;
        conclude_transaction(txn, result, Self::raise_error).await
    }
}
