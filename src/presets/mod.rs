//! Provider stacks that compose repository and service behavior.
//!
//! Component keys use absolute `crate::components` paths so a context
//! `extra(...)` / table entry replaces a stack key by token identity,
//! the same way the original `LookupStack` / `CrudStack` did.
//!
//! - [`RepositoryLookupStack`] — identity, insert, get-or-create, upsert,
//!   pagination, and the default policies (unique-columns identity,
//!   primary-key ordering, 20/100 pagination, unrestricted entity filter,
//!   standard unique-conflict detection, no-op write hook).
//! - [`RepositoryCrudStack`] — lookup plus get / update / delete by id.
//! - [`ResourceServiceStack`] — generic input/output service providers.
//!
//! Concrete applications compose the stacks explicitly:
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
//! Error maps stay in the application crate.

pub mod stack;

pub use stack::{RepositoryCrudStack, RepositoryLookupStack, ResourceServiceStack};
