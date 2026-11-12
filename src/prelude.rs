//! Convenient re-exports for application contexts.

pub use cgp_fork::prelude::*;

pub use crate::accessors::{HasDbConn, HasUniqueColumns};
pub use crate::components::conflict::CanDetectUniqueConflict;
pub use crate::components::identity::CanBuildIdentityCondition;
pub use crate::components::pagination::{CanBuildEntityOrder, HasPaginationPolicy};
pub use crate::components::repository::{
    CanDeleteEntityById, CanFindEntityByIdentity, CanGetEntityById, CanInsertEntity,
    CanInsertOrGetEntity, CanPaginateEntities, CanUpdateEntityById, CanUpsertEntity,
};
pub use crate::components::service::{
    CanApplyUpdateInput, CanCreateResource, CanDecodeCreateInput, CanDeleteResource,
    CanEncodeOutput, CanGetOrCreateResource, CanGetResource, CanListResources, CanUpdateResource,
    HasCreateInputType, HasOutputType, HasUpdateInputType,
};
pub use crate::components::transaction::CanPrepareEntityWrite;
pub use crate::error::{
    ErrConflictFetchFailed, ErrEntityNotFound, ErrIdentityEmpty, ErrIdentityFieldUnset,
    ErrMissingOrdering, ErrMissingPrimaryKey, ErrPrimaryKeyUnset, ErrUniqueConflict,
    ErrUnsupportedKeyArity,
};
pub use crate::pagination::{Page, PageRequest, ResolvedPage};
pub use crate::presets::{RepositoryCrudStack, RepositoryLookupStack, ResourceServiceStack};
pub use crate::types::{
    ActiveModelOf, ColumnOf, HasEntityType, InsertOutcome, ModelOf, PrimaryKeyOf, UpsertOutcome,
};
