use cgp_fork::prelude::*;
use sea_orm::{EntityTrait, PrimaryKeyTrait};

use crate::accessors::HasDbConn;
use crate::components::repository::{EntityByIdDeleter, EntityByIdDeleterComponent};
use crate::types::{HasEntityType, PrimaryKeyOf};

/// Delete a row by SeaORM's complete primary-key [`ValueType`].
///
/// Returns `true` if a row was deleted. Does not assume an `i32` key.
pub struct DeleteEntityById;

#[cgp_impl(DeleteEntityById)]
#[uses(HasDbConn, CanRaiseError<sea_orm::DbErr>)]
#[use_type(HasEntityType.Entity, HasErrorType.Error)]
impl EntityByIdDeleter
where
    Self: Send + Sync,
    Error: Send,
    Entity: Send,
    <Entity as EntityTrait>::Model: Send,
    <<Entity as EntityTrait>::PrimaryKey as PrimaryKeyTrait>::ValueType: Send,
{
    async fn delete_by_id(&self, id: PrimaryKeyOf<Self>) -> Result<bool, Error> {
        let result = Entity::delete_by_id(id)
            .exec(self.db_conn())
            .await
            .map_err(Self::raise_error)?;
        Ok(result.rows_affected > 0)
    }
}
