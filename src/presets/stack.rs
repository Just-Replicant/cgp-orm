//! The three reusable stacks.
//!
//! `cgp_preset!` expands to modules; those generated items are documented
//! here and in `presets/mod.rs`.
#![allow(missing_docs)] // generated preset modules
#![allow(clippy::crate_in_macro_def)] // cgp_preset! records cgp_orm:: component keys

use cgp_fork::prelude::*;

use cgp_orm::providers::conflict::StandardUniqueConflict;
use cgp_orm::providers::identity::UniqueColumnsIdentity;
use cgp_orm::providers::pagination::{DefaultPaginationPolicy, PrimaryKeyOrdering};
use cgp_orm::providers::repository::{
    DeleteEntityById, FindEntityByIdentity, GetEntityById, GetOrCreateEntity, InsertEntity,
    PaginateEntities, UpdateEntityById, UpsertEntity,
};
use cgp_orm::providers::service::{
    CreateResourceProvider, DeleteResourceProvider, GetOrCreateResourceProvider,
    GetResourceProvider, ListResourcesProvider, UpdateResourceProvider,
};
use cgp_orm::providers::transaction::NoopPrepareEntityWrite;

// Contexts still provide HasEntityType, HasUniqueColumns (for the default
// identity), and an error map. Override IdentityConditionBuilderComponent
// for a custom identity, and PaginationPolicyComponent for custom sizes.
cgp_preset! {
    RepositoryLookupStack {
        cgp_orm::components::EntityPaginatorComponent:
            PaginateEntities,
        cgp_orm::components::EntityIdentityFinderComponent:
            FindEntityByIdentity,
        cgp_orm::components::EntityInserterComponent:
            InsertEntity,
        cgp_orm::components::EntityGetOrCreatorComponent:
            GetOrCreateEntity,
        cgp_orm::components::EntityUpserterComponent:
            UpsertEntity,
        cgp_orm::components::EntityWritePreparerComponent:
            NoopPrepareEntityWrite,
        cgp_orm::components::IdentityConditionBuilderComponent:
            UniqueColumnsIdentity,
        cgp_orm::components::EntityOrderingComponent:
            PrimaryKeyOrdering,
        cgp_orm::components::PaginationPolicyComponent:
            DefaultPaginationPolicy,
        cgp_orm::components::UniqueConflictDetectorComponent:
            StandardUniqueConflict,
    }
}

// Parent is a full path so replaying the preset still resolves LookupStack.
cgp_preset! {
    RepositoryCrudStack: cgp_orm::presets::stack::RepositoryLookupStack {
        cgp_orm::components::EntityByIdGetterComponent:
            GetEntityById,
        cgp_orm::components::EntityByIdUpdaterComponent:
            UpdateEntityById,
        cgp_orm::components::EntityByIdDeleterComponent:
            DeleteEntityById,
    }
}

// Contexts provide CreateInput / UpdateInput / Output and the three codecs.
// Compose with RepositoryCrudStack at the application preset:
// `RepositoryCrudStack + ResourceServiceStack`.
cgp_preset! {
    ResourceServiceStack {
        cgp_orm::components::CreateResourceComponent:
            CreateResourceProvider,
        cgp_orm::components::GetOrCreateResourceComponent:
            GetOrCreateResourceProvider,
        cgp_orm::components::UpdateResourceComponent:
            UpdateResourceProvider,
        cgp_orm::components::GetResourceComponent:
            GetResourceProvider,
        cgp_orm::components::ListResourcesComponent:
            ListResourcesProvider,
        cgp_orm::components::DeleteResourceComponent:
            DeleteResourceProvider,
    }
}
