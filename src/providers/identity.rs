//! [`UniqueColumnsIdentity`]: build an identity condition from a column list.

use std::fmt::Debug;

use cgp_fork::prelude::*;
use sea_orm::sea_query::Condition;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait};

use crate::accessors::HasUniqueColumns;
use crate::components::identity::{IdentityConditionBuilder, IdentityConditionBuilderComponent};
use crate::error::{ErrIdentityEmpty, ErrIdentityFieldUnset};
use crate::types::{ActiveModelOf, ColumnOf, HasEntityType};

/// Reusable identity policy: `AND` equality predicates for
/// [`HasUniqueColumns::unique_columns`].
///
/// Entities that need a more complex identity (disjunctions, partial
/// uniques, expressions) implement [`IdentityConditionBuilder`] themselves
/// and override this provider in their preset.
pub struct UniqueColumnsIdentity;

#[cgp_impl(UniqueColumnsIdentity)]
#[uses(
    HasUniqueColumns,
    CanRaiseError<ErrIdentityEmpty>,
    CanRaiseError<ErrIdentityFieldUnset<ColumnOf<Self>>>
)]
#[use_type(HasEntityType.Entity, HasErrorType.Error)]
impl IdentityConditionBuilder
where
    <Entity as EntityTrait>::ActiveModel: ActiveModelTrait<Entity = Entity>,
    ColumnOf<Self>: ColumnTrait + Copy + Debug,
{
    fn identity_condition(&self, model: &ActiveModelOf<Self>) -> Result<Condition, Error> {
        condition_from_columns::<Entity, _>(model, self.unique_columns()).map_err(|err| match err {
            ColumnLookupError::Empty => Self::raise_error(ErrIdentityEmpty),
            ColumnLookupError::Unset(column) => Self::raise_error(ErrIdentityFieldUnset { column }),
        })
    }
}

/// Failure when building a condition from a column list.
pub(crate) enum ColumnLookupError<Column> {
    /// The column list was empty.
    Empty,
    /// A listed column was `NotSet` on the model.
    Unset(Column),
}

/// `AND` together `column = value` predicates for every column in `columns`.
pub(crate) fn condition_from_columns<E, I>(
    model: &E::ActiveModel,
    columns: I,
) -> Result<Condition, ColumnLookupError<E::Column>>
where
    E: EntityTrait,
    E::ActiveModel: ActiveModelTrait<Entity = E>,
    E::Column: ColumnTrait + Copy + Debug,
    I: IntoIterator<Item = E::Column>,
{
    let mut condition = Condition::all();
    let mut has_column = false;

    for column in columns {
        has_column = true;
        let value = model
            .get(column)
            .into_value()
            .ok_or(ColumnLookupError::Unset(column))?;
        condition = condition.add(column.eq(value));
    }

    if !has_column {
        return Err(ColumnLookupError::Empty);
    }

    Ok(condition)
}
