//! Transport-independent SeaORM repository and resource-service components
//! for [Context-Generic Programming](https://contextgeneric.dev) (`cgp-fork`).
//!
//! # What this crate is
//!
//! `cgp-orm` sits between a SeaORM entity and an application service. It does
//! **not** depend on a transport stack or any application entity. Contexts
//! choose policies (identity, pagination, ordering, entity filter, conflict,
//! transaction hooks, error mapping) by wiring providers through CGP presets.
//!
//! The design principle: **presets compose behavior, contexts choose policies,
//! and macros do not own architecture.**
//!
//! # Primary keys
//!
//! Repository APIs use SeaORM's own primary-key value type:
//!
//! ```ignore
//! pub type PrimaryKeyOf<C> =
//!     <<<C as HasEntityType>::Entity as EntityTrait>::PrimaryKey as PrimaryKeyTrait>::ValueType;
//!
//! async fn get_by_id(&self, id: PrimaryKeyOf<Self>) -> Result<Option<ModelOf<Self>>, Error>;
//! async fn delete_by_id(&self, id: PrimaryKeyOf<Self>) -> Result<bool, Error>;
//! ```
//!
//! This supports `i32`, `Uuid`, and composite keys with no `From<i32>` bound.
//! Update merging excludes **every** primary-key column; nothing assumes the
//! first primary-key column is the identity.
//!
//! # Repository versus service
//!
//! The repository layer talks SeaORM models:
//! [`CanGetEntityById`], [`CanInsertEntity`], [`CanUpdateEntityById`],
//! [`CanDeleteEntityById`], [`CanFindEntityByIdentity`], [`CanPaginateEntities`],
//! [`CanGetOrCreateEntity`], and [`CanUpsertEntity`].
//!
//! The service layer talks context-chosen input and output types
//! ([`HasCreateInputType`], [`HasUpdateInputType`], [`HasOutputType`]) via
//! [`CanDecodeCreateInput`], [`CanApplyUpdateInput`], [`CanEncodeOutput`],
//! [`CanCreateResource`], [`CanUpdateResource`],
//! [`CanGetResource`], [`CanListResources`], and [`CanDeleteResource`].
//!
//! # Write semantics
//!
//! There is no generic `find_or_insert` that implies atomicity. The explicit
//! operations are:
//!
//! - [`CanInsertEntity::insert`] — insert, fail on unique conflict.
//! - [`CanFindEntityByIdentity::find_by_identity`] — look up by the identity policy.
//! - [`CanGetOrCreateEntity::get_or_create`] — return the row for this identity,
//!   inserting only when it is absent. A concurrent unique-constraint conflict
//!   rolls back and fetches the existing row. This relies on a **database
//!   unique constraint**; a transaction around find-then-insert does not
//!   prevent concurrent duplicates by itself.
//! - [`CanUpsertEntity::upsert`] — merge-update the existing row, or insert when
//!   it is absent. A concurrent unique conflict is merged on a clean
//!   connection after rollback.
//!
//! Transaction completion [preserves the original operation error](crate::transaction)
//! if rollback also fails.
//!
//! # Stacks
//!
//! - [`presets::RepositoryLookupStack`] — identity lookup, insert, get-or-create,
//!   upsert, pagination, and default policies.
//! - [`presets::RepositoryCrudStack`] — lookup plus get / update / delete by id.
//! - [`presets::ResourceServiceStack`] — generic input/output service providers.
//!
//! An application composes them explicitly:
//!
//! ```ignore
//! cgp_preset! {
//!     PostBase: RepositoryCrudStack + ResourceServiceStack {
//!         EntityTypeProviderComponent: UseType<post::Entity>,
//!         CreateInputTypeProviderComponent: UseType<CreatePost>,
//!         UpdateInputTypeProviderComponent: UseType<UpdatePost>,
//!         OutputTypeProviderComponent: UseType<PostOutput>,
//!         // codecs, identity, and hooks
//!     }
//! }
//! ```
//!
//! Error maps are application policy and do not live here.
//!
//! # Pagination and filters
//!
//! Offset pagination through [`PageRequest`] is implemented now. The
//! [`CanBuildEntityFilter`] component supplies the WHERE clause
//! (default: no restriction). [`PageRequest::without_total`] skips
//! `COUNT(*)` and leaves [`Page::total`] as `None`. Keyset/cursor
//! pagination is a documented extension point — see
//! [`pagination`] and `ARCHITECTURE.md` — and can be added later without
//! changing the offset API. Soft delete is an application filter, not a
//! library deletion policy.

#![deny(missing_docs)]
#![forbid(unsafe_code)]
#![cfg_attr(docsrs, feature(doc_cfg))]

// So preset keys (`cgp_orm::components::…`) resolve both inside this
// crate and when a consumer replays the stacks.
extern crate self as cgp_orm;

pub mod accessors;
pub mod components;
pub mod error;
pub mod pagination;
pub mod prelude;
pub mod presets;
pub mod providers;
pub mod transaction;
pub mod types;
pub mod util;

pub use prelude::*;
