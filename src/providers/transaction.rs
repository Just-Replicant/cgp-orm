//! Default no-op write hook.

use cgp_fork::prelude::*;
use sea_orm::{DatabaseTransaction, EntityTrait};

use crate::components::transaction::{EntityWritePreparer, EntityWritePreparerComponent};
use crate::types::{ActiveModelOf, HasEntityType};

/// A write hook that does nothing. Safe default for entities with no
/// pre-write side effects.
pub struct NoopPrepareEntityWrite;

#[cgp_impl(NoopPrepareEntityWrite)]
#[use_type(HasEntityType.Entity, HasErrorType.Error)]
impl EntityWritePreparer
where
    Self: Sync,
    <Entity as EntityTrait>::ActiveModel: Sync,
{
    async fn prepare_entity_write(
        &self,
        _txn: &DatabaseTransaction,
        _model: &ActiveModelOf<Self>,
    ) -> Result<(), Error> {
        Ok(())
    }
}
