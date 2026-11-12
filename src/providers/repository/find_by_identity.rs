use cgp_fork::prelude::*;
use sea_orm::{EntityTrait, QueryFilter};

use crate::accessors::HasDbConn;
use crate::components::identity::CanBuildIdentityCondition;
use crate::components::repository::{EntityIdentityFinder, EntityIdentityFinderComponent};
use crate::types::{ActiveModelOf, HasEntityType, ModelOf};

/// Look up a row using the context's identity policy.
pub struct FindEntityByIdentity;

#[cgp_impl(FindEntityByIdentity)]
#[uses(HasDbConn, CanBuildIdentityCondition, CanRaiseError<sea_orm::DbErr>)]
#[use_type(HasEntityType.Entity, HasErrorType.Error)]
impl EntityIdentityFinder
where
    Self: Send + Sync,
    Error: Send,
    Entity: Send,
    <Entity as EntityTrait>::Model: Send,
    <Entity as EntityTrait>::ActiveModel: Sync,
{
    async fn find_by_identity(
        &self,
        model: &ActiveModelOf<Self>,
    ) -> Result<Option<ModelOf<Self>>, Error> {
        let condition = self.identity_condition(model)?;
        Entity::find()
            .filter(condition)
            .one(self.db_conn())
            .await
            .map_err(Self::raise_error)
    }
}
