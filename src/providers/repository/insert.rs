use cgp_fork::prelude::*;
use sea_orm::{ActiveModelTrait, EntityTrait, IntoActiveModel, TransactionTrait};

use crate::accessors::HasDbConn;
use crate::components::repository::{EntityInserter, EntityInserterComponent};
use crate::components::transaction::CanPrepareEntityWrite;
use crate::transaction::conclude_transaction;
use crate::types::{ActiveModelOf, HasEntityType, ModelOf};

/// Insert a row inside a transaction, running the write hook first.
///
/// Unique conflicts are returned as the original [`sea_orm::DbErr`]; this
/// operation does not fetch an existing row. Use
/// [`crate::providers::InsertOrGetEntity`] for that.
pub struct InsertEntity;

#[cgp_impl(InsertEntity)]
#[uses(
    HasDbConn,
    CanPrepareEntityWrite,
    CanRaiseError<sea_orm::DbErr>
)]
#[use_type(HasEntityType.Entity, HasErrorType.Error)]
impl EntityInserter
where
    Self: Send + Sync,
    Error: Send,
    Entity: Send,
    <Entity as EntityTrait>::Model: IntoActiveModel<<Entity as EntityTrait>::ActiveModel> + Send,
    <Entity as EntityTrait>::ActiveModel: ActiveModelTrait<Entity = Entity> + Send + Sync,
{
    async fn insert(&self, model: ActiveModelOf<Self>) -> Result<ModelOf<Self>, Error> {
        let txn = self.db_conn().begin().await.map_err(Self::raise_error)?;
        let result = async {
            self.prepare_entity_write(&txn, &model).await?;
            model.insert(&txn).await.map_err(Self::raise_error)
        }
        .await;
        conclude_transaction(txn, result, Self::raise_error).await
    }
}
