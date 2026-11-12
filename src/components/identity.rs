//! Identity policy: how a row is looked up by its natural key.
//!
//! This replaces the previous `fn unique_columns(&self) -> Vec<Column>`
//! accessor as the public identity API. A reusable
//! [`crate::providers::UniqueColumnsIdentity`] provider still builds a
//! condition from a column list for the common case. Entities with a more
//! complex identity (partial uniques, `OR` groups, expressions) implement
//! this trait themselves.

use cgp_fork::prelude::*;
use sea_orm::Condition;

use crate::types::{ActiveModelOf, HasEntityType};

/// Build a SeaORM [`Condition`] that identifies `model`.
#[cgp_component(IdentityConditionBuilder)]
#[use_type(HasEntityType.Entity, HasErrorType.Error)]
pub trait CanBuildIdentityCondition {
    /// Return a non-empty condition derived from `model`.
    ///
    /// Returns [`crate::error::ErrIdentityEmpty`] if the policy has no
    /// columns, or [`crate::error::ErrIdentityFieldUnset`] if a required
    /// field is `NotSet`.
    fn identity_condition(&self, model: &ActiveModelOf<Self>) -> Result<Condition, Error>;
}
