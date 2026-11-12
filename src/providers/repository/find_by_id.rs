use cgp_fork::prelude::*;
use sea_orm::{EntityTrait, PrimaryKeyTrait};

use crate::accessors::HasDbConn;
use crate::components::repository::{EntityByIdFinder, EntityByIdFinderComponent};
use crate::types::{HasEntityType, ModelOf, PrimaryKeyOf};

/// Look up a row with SeaORM's complete primary-key [`ValueType`].
pub struct FindEntityById;

#[cgp_impl(FindEntityById)]
#[uses(HasDbConn, CanRaiseError<sea_orm::DbErr>)]
#[use_type(HasEntityType.Entity, HasErrorType.Error)]
impl EntityByIdFinder
where
    Self: Send + Sync,
    Error: Send,
    Entity: Send,
    <Entity as EntityTrait>::Model: Send,
    <<Entity as EntityTrait>::PrimaryKey as PrimaryKeyTrait>::ValueType: Send,
{
    async fn find_by_id(&self, id: PrimaryKeyOf<Self>) -> Result<Option<ModelOf<Self>>, Error> {
        Entity::find_by_id(id)
            .one(self.db_conn())
            .await
            .map_err(Self::raise_error)
    }
}
