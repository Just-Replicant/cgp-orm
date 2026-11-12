//! Generic resource-service providers. Codecs are supplied by the context.

mod create;
mod delete;
mod get;
mod get_or_create;
mod list;
mod update;

pub use create::CreateResourceProvider;
pub use delete::DeleteResourceProvider;
pub use get::GetResourceProvider;
pub use get_or_create::GetOrCreateResourceProvider;
pub use list::ListResourcesProvider;
pub use update::UpdateResourceProvider;
