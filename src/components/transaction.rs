//! Transaction hooks run inside the write transaction, before insert or update.
//!
//! The default provider is [`crate::providers::NoopPrepareEntityWrite`].
//! Application contexts replace it — for example to clear other "current"
//! device rows before inserting a new current device.

use cgp_fork::prelude::*;
use sea_orm::DatabaseTransaction;

use crate::types::{ActiveModelOf, HasEntityType};

/// A hook invoked inside a write transaction, before the insert or update.
#[cgp_component(EntityWritePreparer)]
#[cgp_async(Send)]
#[use_type(HasEntityType.Entity, HasErrorType.Error)]
pub trait CanPrepareEntityWrite {
    /// Prepare the transaction for writing `model`.
    ///
    /// The hook runs in the same transaction as the write. If the write
    /// later fails and the transaction is rolled back, side effects of this
    /// hook are rolled back with it.
    async fn prepare_entity_write(
        &self,
        txn: &DatabaseTransaction,
        model: &ActiveModelOf<Self>,
    ) -> Result<(), Error>;
}
