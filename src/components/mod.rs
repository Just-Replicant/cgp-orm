//! CGP component traits for the repository, identity, transaction, pagination,
//! conflict, and resource-service layers.

pub mod conflict;
pub mod identity;
pub mod pagination;
pub mod repository;
pub mod service;
pub mod transaction;

pub use conflict::{
    CanDetectUniqueConflict, UniqueConflictDetector, UniqueConflictDetectorComponent,
};
pub use identity::{
    CanBuildIdentityCondition, IdentityConditionBuilder, IdentityConditionBuilderComponent,
};
pub use pagination::{
    CanBuildEntityOrder, EntityOrdering, EntityOrderingComponent, HasPaginationPolicy,
    PaginationPolicy, PaginationPolicyComponent,
};
pub use repository::{
    CanDeleteEntityById, CanFindEntityByIdentity, CanGetEntityById, CanGetOrCreateEntity,
    CanInsertEntity, CanPaginateEntities, CanUpdateEntityById, CanUpsertEntity, EntityByIdDeleter,
    EntityByIdDeleterComponent, EntityByIdGetter, EntityByIdGetterComponent, EntityByIdUpdater,
    EntityByIdUpdaterComponent, EntityGetOrCreator, EntityGetOrCreatorComponent,
    EntityIdentityFinder, EntityIdentityFinderComponent, EntityInserter, EntityInserterComponent,
    EntityPaginator, EntityPaginatorComponent, EntityUpserter, EntityUpserterComponent,
};
pub use service::{
    ApplyUpdateInput, ApplyUpdateInputComponent, CanApplyUpdateInput, CanCreateResource,
    CanDecodeCreateInput, CanDeleteResource, CanEncodeOutput, CanGetResource, CanListResources,
    CanUpdateResource, CreateResource, CreateResourceComponent, DecodeCreateInput,
    DecodeCreateInputComponent, DeleteResource, DeleteResourceComponent, EncodeOutput,
    EncodeOutputComponent, GetResource, GetResourceComponent, HasCreateInputType, HasOutputType,
    HasUpdateInputType,
    ListResources, ListResourcesComponent, UpdateResource, UpdateResourceComponent,
};
pub use transaction::{CanPrepareEntityWrite, EntityWritePreparer, EntityWritePreparerComponent};
