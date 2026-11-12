//! Shared test contexts, codecs, and database setup.

pub mod entities;

use cgp_fork::anyhow::{DisplayAnyhowError, RaiseAnyhowError, UseAnyhowError};
use cgp_fork::core::error::{
    ErrorRaiserComponent, ErrorTypeProviderComponent, ErrorWrapperComponent,
};
use cgp_fork::extra::error::{RaiseFrom, RaiseInfallible, ReturnError};
use cgp_fork::prelude::*;
use cgp_orm::accessors::{UniqueColumnsGetter, UniqueColumnsGetterComponent};
use cgp_orm::components::filter::{EntityFilterBuilder, EntityFilterBuilderComponent};
use cgp_orm::components::service::{
    ApplyUpdateInput, ApplyUpdateInputComponent, DecodeCreateInput, DecodeCreateInputComponent,
    EncodeOutput, EncodeOutputComponent, HasCreateInputType, HasOutputType, HasUpdateInputType,
};
use cgp_orm::components::transaction::{EntityWritePreparer, EntityWritePreparerComponent};
use cgp_orm::error::{
    ErrConflictFetchFailed, ErrEntityNotFound, ErrIdentityEmpty, ErrIdentityFieldUnset,
    ErrMissingOrdering, ErrMissingPrimaryKey, ErrPrimaryKeySetFailed, ErrPrimaryKeyUnset,
    ErrUniqueConflict, ErrUnsupportedKeyArity,
};
use cgp_orm::presets::{RepositoryCrudStack, ResourceServiceStack};
use cgp_orm::providers::ConstPaginationPolicy;
use cgp_orm::providers::conflict::{MessageUniqueConflict, OrUniqueConflict, SqlErrUniqueConflict};
use cgp_orm::types::HasEntityType;
use sea_orm::{
    ColumnTrait, Condition, ConnectionTrait, Database, DatabaseConnection, DatabaseTransaction,
    EntityTrait, NotSet, Schema, Set,
};
use thiserror::Error;

use entities::{document, item, membership};

// ---------------------------------------------------------------------------
// Error map (anyhow). Application crates own their maps; this is tests only.
// ---------------------------------------------------------------------------

pub struct TestErrors;

delegate_components! {
    #[unchecked]
    TestErrors {
        open {
            ErrorRaiserComponent,
            ErrorWrapperComponent,
        };

        ErrorTypeProviderComponent: UseAnyhowError,
        @ErrorRaiserComponent.anyhow::Error: ReturnError,
        @ErrorRaiserComponent.core::convert::Infallible: RaiseInfallible,
        @ErrorRaiserComponent.[::sea_orm::DbErr]: RaiseFrom,
        @ErrorRaiserComponent.[
            String,
            HookFailed,
            ErrIdentityEmpty,
            ErrMissingPrimaryKey,
            ErrPrimaryKeySetFailed,
            ErrUnsupportedKeyArity,
            ErrMissingOrdering,
            ErrConflictFetchFailed,
            ErrEntityNotFound,
            ErrUniqueConflict,
        ]: DisplayAnyhowError,
        @ErrorRaiserComponent.<'a> &'a str: DisplayAnyhowError,
        @ErrorRaiserComponent.<Column> ErrIdentityFieldUnset<Column>: DisplayAnyhowError,
        @ErrorRaiserComponent.<Column> ErrPrimaryKeyUnset<Column>: DisplayAnyhowError,
        @ErrorWrapperComponent.String: RaiseAnyhowError,
        @ErrorWrapperComponent.<'a> &'a str: DisplayAnyhowError,
    }
}

/// A write-hook failure used to test rollback of the original error.
#[derive(Debug, Clone, Copy, Error)]
#[error("hook failed: {0}")]
pub struct HookFailed(pub &'static str);

// ---------------------------------------------------------------------------
// Inputs / outputs and codecs (distinct create / update / output types).
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemInput {
    pub name: String,
    pub value: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemOutput {
    pub id: i32,
    pub name: String,
    pub value: i32,
}

pub struct ItemCodec;

#[cgp_impl(ItemCodec)]
#[use_type(
    HasEntityType.{Entity = item::Entity},
    HasErrorType.Error,
    HasCreateInputType.{CreateInput = ItemInput}
)]
impl DecodeCreateInput {
    fn decode_create_input(&self, input: ItemInput) -> Result<item::ActiveModel, Error> {
        Ok(item::ActiveModel {
            id: NotSet,
            name: Set(input.name),
            value: Set(input.value),
        })
    }
}

#[cgp_impl(ItemCodec)]
#[use_type(
    HasEntityType.{Entity = item::Entity},
    HasErrorType.Error,
    HasUpdateInputType.{UpdateInput = ItemInput}
)]
impl ApplyUpdateInput {
    fn apply_update_input(&self, id: i32, input: ItemInput) -> Result<item::ActiveModel, Error> {
        Ok(item::ActiveModel {
            id: Set(id),
            name: Set(input.name),
            value: Set(input.value),
        })
    }
}

#[cgp_impl(ItemCodec)]
#[use_type(
    HasEntityType.{Entity = item::Entity},
    HasErrorType.Error,
    HasOutputType.{Output = ItemOutput}
)]
impl EncodeOutput {
    fn encode_output(&self, model: item::Model) -> Result<ItemOutput, Error> {
        Ok(ItemOutput {
            id: model.id,
            name: model.name,
            value: model.value,
        })
    }
}

pub struct ItemUniqueColumns;

#[cgp_impl(ItemUniqueColumns)]
#[use_type(HasEntityType.{Entity = item::Entity})]
impl UniqueColumnsGetter {
    fn unique_columns(&self) -> Vec<item::Column> {
        vec![item::Column::Name]
    }
}

// Documents (UUID PK)

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentInput {
    pub id: uuid::Uuid,
    pub title: String,
    pub body: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentOutput {
    pub id: uuid::Uuid,
    pub title: String,
    pub body: String,
}

pub struct DocumentCodec;

#[cgp_impl(DocumentCodec)]
#[use_type(
    HasEntityType.{Entity = document::Entity},
    HasErrorType.Error,
    HasCreateInputType.{CreateInput = DocumentInput}
)]
impl DecodeCreateInput {
    fn decode_create_input(&self, input: DocumentInput) -> Result<document::ActiveModel, Error> {
        Ok(document::ActiveModel {
            id: Set(input.id),
            title: Set(input.title),
            body: Set(input.body),
        })
    }
}

#[cgp_impl(DocumentCodec)]
#[use_type(
    HasEntityType.{Entity = document::Entity},
    HasErrorType.Error,
    HasUpdateInputType.{UpdateInput = DocumentInput}
)]
impl ApplyUpdateInput {
    fn apply_update_input(
        &self,
        id: uuid::Uuid,
        input: DocumentInput,
    ) -> Result<document::ActiveModel, Error> {
        Ok(document::ActiveModel {
            id: Set(id),
            title: Set(input.title),
            body: Set(input.body),
        })
    }
}

#[cgp_impl(DocumentCodec)]
#[use_type(
    HasEntityType.{Entity = document::Entity},
    HasErrorType.Error,
    HasOutputType.{Output = DocumentOutput}
)]
impl EncodeOutput {
    fn encode_output(&self, model: document::Model) -> Result<DocumentOutput, Error> {
        Ok(DocumentOutput {
            id: model.id,
            title: model.title,
            body: model.body,
        })
    }
}

pub struct DocumentUniqueColumns;

#[cgp_impl(DocumentUniqueColumns)]
#[use_type(HasEntityType.{Entity = document::Entity})]
impl UniqueColumnsGetter {
    fn unique_columns(&self) -> Vec<document::Column> {
        vec![document::Column::Title]
    }
}

// Memberships (composite PK)

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MembershipInput {
    pub user_id: i32,
    pub group_id: i32,
    pub role: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MembershipOutput {
    pub user_id: i32,
    pub group_id: i32,
    pub role: String,
}

pub struct MembershipCodec;

#[cgp_impl(MembershipCodec)]
#[use_type(
    HasEntityType.{Entity = membership::Entity},
    HasErrorType.Error,
    HasCreateInputType.{CreateInput = MembershipInput}
)]
impl DecodeCreateInput {
    fn decode_create_input(
        &self,
        input: MembershipInput,
    ) -> Result<membership::ActiveModel, Error> {
        Ok(membership::ActiveModel {
            user_id: Set(input.user_id),
            group_id: Set(input.group_id),
            role: Set(input.role),
        })
    }
}

#[cgp_impl(MembershipCodec)]
#[use_type(
    HasEntityType.{Entity = membership::Entity},
    HasErrorType.Error,
    HasUpdateInputType.{UpdateInput = MembershipInput}
)]
impl ApplyUpdateInput {
    fn apply_update_input(
        &self,
        id: (i32, i32),
        input: MembershipInput,
    ) -> Result<membership::ActiveModel, Error> {
        Ok(membership::ActiveModel {
            user_id: Set(id.0),
            group_id: Set(id.1),
            role: Set(input.role),
        })
    }
}

#[cgp_impl(MembershipCodec)]
#[use_type(
    HasEntityType.{Entity = membership::Entity},
    HasErrorType.Error,
    HasOutputType.{Output = MembershipOutput}
)]
impl EncodeOutput {
    fn encode_output(&self, model: membership::Model) -> Result<MembershipOutput, Error> {
        Ok(MembershipOutput {
            user_id: model.user_id,
            group_id: model.group_id,
            role: model.role,
        })
    }
}

pub struct MembershipUniqueColumns;

#[cgp_impl(MembershipUniqueColumns)]
#[use_type(HasEntityType.{Entity = membership::Entity})]
impl UniqueColumnsGetter {
    fn unique_columns(&self) -> Vec<membership::Column> {
        vec![membership::Column::UserId, membership::Column::GroupId]
    }
}

// ---------------------------------------------------------------------------
// Presets and contexts
// ---------------------------------------------------------------------------

cgp_preset! {
    ItemBase: RepositoryCrudStack + ResourceServiceStack {
        cgp_orm::types::EntityTypeProviderComponent:
            UseType<item::Entity>,
        cgp_orm::components::service::CreateInputTypeProviderComponent:
            UseType<ItemInput>,
        cgp_orm::components::service::UpdateInputTypeProviderComponent:
            UseType<ItemInput>,
        cgp_orm::components::service::OutputTypeProviderComponent:
            UseType<ItemOutput>,
        [
            cgp_orm::components::DecodeCreateInputComponent,
            cgp_orm::components::ApplyUpdateInputComponent,
            cgp_orm::components::EncodeOutputComponent,
        ]: ItemCodec,
        cgp_orm::accessors::UniqueColumnsGetterComponent: ItemUniqueColumns,
    }
}

cgp_preset! {
    DocumentBase: RepositoryCrudStack + ResourceServiceStack {
        cgp_orm::types::EntityTypeProviderComponent:
            UseType<document::Entity>,
        cgp_orm::components::service::CreateInputTypeProviderComponent:
            UseType<DocumentInput>,
        cgp_orm::components::service::UpdateInputTypeProviderComponent:
            UseType<DocumentInput>,
        cgp_orm::components::service::OutputTypeProviderComponent:
            UseType<DocumentOutput>,
        [
            cgp_orm::components::DecodeCreateInputComponent,
            cgp_orm::components::ApplyUpdateInputComponent,
            cgp_orm::components::EncodeOutputComponent,
        ]: DocumentCodec,
        cgp_orm::accessors::UniqueColumnsGetterComponent: DocumentUniqueColumns,
    }
}

cgp_preset! {
    MembershipBase: RepositoryCrudStack + ResourceServiceStack {
        cgp_orm::types::EntityTypeProviderComponent:
            UseType<membership::Entity>,
        cgp_orm::components::service::CreateInputTypeProviderComponent:
            UseType<MembershipInput>,
        cgp_orm::components::service::UpdateInputTypeProviderComponent:
            UseType<MembershipInput>,
        cgp_orm::components::service::OutputTypeProviderComponent:
            UseType<MembershipOutput>,
        [
            cgp_orm::components::DecodeCreateInputComponent,
            cgp_orm::components::ApplyUpdateInputComponent,
            cgp_orm::components::EncodeOutputComponent,
        ]: MembershipCodec,
        cgp_orm::accessors::UniqueColumnsGetterComponent: MembershipUniqueColumns,
    }
}

macro_rules! error_forward {
    ($preset:ident, $base:ident) => {
        cgp_preset! {
            $preset: $base {
                ErrorTypeProviderComponent:
                    <TestErrors as DelegateComponent<ErrorTypeProviderComponent>>::Delegate,
                ErrorRaiserComponent:
                    <TestErrors as DelegateComponent<ErrorRaiserComponent>>::Delegate,
                ErrorWrapperComponent:
                    <TestErrors as DelegateComponent<ErrorWrapperComponent>>::Delegate,
            }
        }
    };
}

error_forward!(ItemAppPreset, ItemBase);
error_forward!(DocumentAppPreset, DocumentBase);
error_forward!(MembershipAppPreset, MembershipBase);

#[derive(HasField, Clone)]
pub struct ItemApp {
    pub db_conn: DatabaseConnection,
}

#[derive(HasField, Clone)]
pub struct DocumentApp {
    pub db_conn: DatabaseConnection,
}

#[derive(HasField, Clone)]
pub struct MembershipApp {
    pub db_conn: DatabaseConnection,
}

#[derive(HasField, Clone)]
pub struct TightPageItemApp {
    pub db_conn: DatabaseConnection,
}

#[derive(HasField, Clone)]
pub struct FailingItemApp {
    pub db_conn: DatabaseConnection,
}

macro_rules! wire {
    ($app:ident, $preset:ident $(, $extra:tt)*) => {
        impl $app {
            pub fn new(db_conn: DatabaseConnection) -> Self {
                Self { db_conn }
            }
        }

        delegate_components! {
            #[unchecked]
            $app {
                preset $preset,
                $($extra)*
            }
        }
    };
}

wire!(ItemApp, ItemAppPreset);
wire!(DocumentApp, DocumentAppPreset);
wire!(MembershipApp, MembershipAppPreset);

impl TightPageItemApp {
    pub fn new(db_conn: DatabaseConnection) -> Self {
        Self { db_conn }
    }
}

delegate_components! {
    #[unchecked]
    TightPageItemApp {
        preset ItemAppPreset,
        cgp_orm::components::PaginationPolicyComponent: ConstPaginationPolicy<10, 3>,
    }
}

impl FailingItemApp {
    pub fn new(db_conn: DatabaseConnection) -> Self {
        Self { db_conn }
    }
}

delegate_components! {
    #[unchecked]
    FailingItemApp {
        preset ItemAppPreset,
        cgp_orm::components::EntityWritePreparerComponent: FailingPrepare,
    }
}

#[derive(HasField, Clone)]
pub struct FilteredItemApp {
    pub db_conn: DatabaseConnection,
}

impl FilteredItemApp {
    pub fn new(db_conn: DatabaseConnection) -> Self {
        Self { db_conn }
    }
}

/// Restricts item queries to a single `value`, standing in for a tenant id
/// (or a soft-delete / search predicate). The library does not ship a
/// `DeletionPolicy`; the application supplies this filter.
pub struct ItemValueFilter<const VALUE: i32>;

#[cgp_impl(ItemValueFilter<VALUE>)]
#[use_type(HasEntityType.{Entity = item::Entity}, HasErrorType.Error)]
impl<const VALUE: i32> EntityFilterBuilder {
    fn entity_filter(&self) -> Result<Condition, Error> {
        Ok(Condition::all().add(item::Column::Value.eq(VALUE)))
    }
}

delegate_components! {
    #[unchecked]
    FilteredItemApp {
        preset ItemAppPreset,
        cgp_orm::components::EntityFilterBuilderComponent: ItemValueFilter<1>,
    }
}

/// Wires [`OrUniqueConflict`] directly as the provider (not `UseDelegate`).
#[derive(HasField, Clone)]
pub struct SqlErrOrMessageItemApp {
    pub db_conn: DatabaseConnection,
}

impl SqlErrOrMessageItemApp {
    pub fn new(db_conn: DatabaseConnection) -> Self {
        Self { db_conn }
    }
}

delegate_components! {
    #[unchecked]
    SqlErrOrMessageItemApp {
        preset ItemAppPreset,
        cgp_orm::components::UniqueConflictDetectorComponent:
            OrUniqueConflict<SqlErrUniqueConflict, MessageUniqueConflict>,
    }
}

/// Write hook that always fails, to exercise rollback.
pub struct FailingPrepare;

#[cgp_impl(FailingPrepare)]
#[uses(CanRaiseError<HookFailed>)]
#[use_type(HasEntityType.{Entity = item::Entity}, HasErrorType.Error)]
impl EntityWritePreparer
where
    Self: Sync,
    item::ActiveModel: Sync,
{
    async fn prepare_entity_write(
        &self,
        _txn: &DatabaseTransaction,
        _model: &item::ActiveModel,
    ) -> Result<(), Error> {
        Err(Self::raise_error(HookFailed(
            "write hook refused the operation",
        )))
    }
}

// ---------------------------------------------------------------------------
// Database
// ---------------------------------------------------------------------------

pub async fn connect() -> DatabaseConnection {
    Database::connect("sqlite::memory:")
        .await
        .expect("in-memory sqlite")
}

pub async fn create_table<E: EntityTrait>(db: &DatabaseConnection, entity: E) {
    let schema = Schema::new(db.get_database_backend());
    let table = schema.create_table_from_entity(entity);
    db.execute(&table).await.expect("create table");
    for index in schema.create_index_from_entity(entity) {
        db.execute(&index).await.expect("create index");
    }
}

pub async fn setup_items() -> (ItemApp, DatabaseConnection) {
    let db = connect().await;
    create_table(&db, item::Entity).await;
    (ItemApp::new(db.clone()), db)
}

pub async fn setup_documents() -> (DocumentApp, DatabaseConnection) {
    let db = connect().await;
    create_table(&db, document::Entity).await;
    (DocumentApp::new(db.clone()), db)
}

pub async fn setup_memberships() -> (MembershipApp, DatabaseConnection) {
    let db = connect().await;
    create_table(&db, membership::Entity).await;
    (MembershipApp::new(db.clone()), db)
}
