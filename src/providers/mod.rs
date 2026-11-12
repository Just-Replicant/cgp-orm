//! Default providers for every component in this crate.
//!
//! Contexts may replace any of these through a preset override.

pub mod conflict;
pub mod identity;
pub mod pagination;
pub mod repository;
pub mod service;
pub mod transaction;

pub use conflict::{
    MessageUniqueConflict, MysqlUniqueConflict, OrUniqueConflict, PostgresUniqueConflict,
    SqlErrUniqueConflict, SqliteUniqueConflict, StandardUniqueConflict,
    is_unique_constraint_violation, message_is_unique, mysql_unique_message,
    postgres_unique_message, sql_err_is_unique, sqlite_unique_message,
};
pub use identity::UniqueColumnsIdentity;
pub use pagination::{ConstPaginationPolicy, DefaultPaginationPolicy, PrimaryKeyOrdering};
pub use repository::{
    DeleteEntityById, FindEntityByIdentity, GetEntityById, GetOrCreateEntity, InsertEntity,
    PaginateEntities, UpdateEntityById, UpsertEntity,
};
pub use service::{
    CreateResourceProvider, DeleteResourceProvider, GetOrCreateResourceProvider,
    GetResourceProvider, ListResourcesProvider, UpdateResourceProvider,
};
pub use transaction::NoopPrepareEntityWrite;
