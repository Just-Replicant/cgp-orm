use cgp_fork::prelude::*;
use sea_orm::{EntityTrait, QueryFilter};

use crate::accessors::HasDbConn;
use crate::components::filter::CanBuildEntityFilter;
use crate::components::identity::CanBuildIdentityCondition;
use crate::components::repository::{EntityIdentityFinder, EntityIdentityFinderComponent};
use crate::types::{ActiveModelOf, HasEntityType, ModelOf};

/// Look up a row using the context's identity policy and entity filter.
///
/// The entity filter (default: no restriction) is ANDed onto the identity
/// condition. Under [`crate::providers::NoEntityFilter`] this is a no-op,
/// so existing contexts behave as before. An application that supplies a
/// tenant or soft-delete filter will not see rows excluded by that filter,
/// including during `get_or_create` conflict recovery. If a unique
/// constraint fires on a filtered-out row, recovery surfaces
/// [`crate::error::ErrConflictFetchFailed`]; that is an application
/// concern (for example, include deleted rows only in the identity
/// policy, or handle the error).
pub struct FindEntityByIdentity;

#[cgp_impl(FindEntityByIdentity)]
#[uses(
    HasDbConn,
    CanBuildIdentityCondition,
    CanBuildEntityFilter,
    CanRaiseError<sea_orm::DbErr>
)]
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
        Entity::find()
            .filter(self.entity_filter()?)
            .filter(self.identity_condition(model)?)
            .one(self.db_conn())
            .await
            .map_err(Self::raise_error)
    }
}
