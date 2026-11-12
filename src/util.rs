//! Shared SeaORM helpers: composite primary keys and update merging.
//!
//! [`merge_update`] copies **non-primary-key** columns only. Every primary-key
//! column is excluded — this crate never assumes the first primary key is
//! the identity, and it never calls a `primary_key_column()` that returns
//! only the first key.

use std::fmt::Debug;

use sea_orm::entity::{
    ActiveModelTrait, ColumnTrait, EntityTrait, IdenStatic, Iterable, PrimaryKeyToColumn,
    PrimaryKeyTrait,
};
use sea_orm::sea_query::{FromValueTuple, IntoValueTuple, Value, ValueTuple};
use sea_orm::{DbErr, IntoActiveModel};

use crate::error::{ErrMissingPrimaryKey, ErrPrimaryKeyUnset, ErrUnsupportedKeyArity};

/// Failure when reading a complete primary key from an active model.
#[derive(Debug)]
pub enum PrimaryKeyReadError<Column> {
    /// The entity declares no primary-key columns.
    Missing(ErrMissingPrimaryKey),
    /// At least one primary-key column is `NotSet`.
    Unset(ErrPrimaryKeyUnset<Column>),
    /// The key's arity cannot be encoded (currently only zero).
    Arity(ErrUnsupportedKeyArity),
}

/// Collect every primary-key column of `E`.
///
/// The result may be empty only for a misconfigured entity; callers treat
/// that as [`ErrMissingPrimaryKey`].
#[must_use]
pub fn primary_key_columns<E>() -> Vec<E::Column>
where
    E: EntityTrait,
    E::PrimaryKey: PrimaryKeyToColumn<Column = E::Column> + Iterable,
{
    E::PrimaryKey::iter()
        .map(PrimaryKeyToColumn::into_column)
        .collect()
}

/// Whether `column` is one of the entity's primary-key columns.
///
/// Comparison is by SeaORM identifier string so it does not require
/// `PartialEq` on the column enum and does not special-case the first key.
#[must_use]
pub fn is_primary_key_column<E>(column: E::Column) -> bool
where
    E: EntityTrait,
    E::PrimaryKey: PrimaryKeyToColumn<Column = E::Column> + Iterable,
    E::Column: IdenStatic,
{
    primary_key_columns::<E>()
        .into_iter()
        .any(|pk| pk.as_str() == column.as_str())
}

/// Read the complete primary-key [`ValueType`](PrimaryKeyTrait::ValueType)
/// from an active model, including every key column.
pub fn primary_key_from_active_model<E>(
    model: &E::ActiveModel,
) -> Result<
    <<E as EntityTrait>::PrimaryKey as PrimaryKeyTrait>::ValueType,
    PrimaryKeyReadError<E::Column>,
>
where
    E: EntityTrait,
    E::ActiveModel: ActiveModelTrait<Entity = E>,
    E::Column: ColumnTrait + Copy,
    E::PrimaryKey: PrimaryKeyToColumn<Column = E::Column> + Iterable,
{
    let columns = primary_key_columns::<E>();
    if columns.is_empty() {
        return Err(PrimaryKeyReadError::Missing(ErrMissingPrimaryKey::of::<E>()));
    }

    let mut values = Vec::with_capacity(columns.len());
    for column in columns {
        let value = model
            .get(column)
            .into_value()
            .ok_or(PrimaryKeyReadError::Unset(ErrPrimaryKeyUnset { column }))?;
        values.push(value);
    }

    let tuple = value_tuple_from_vec(values).map_err(PrimaryKeyReadError::Arity)?;
    Ok(FromValueTuple::from_value_tuple(tuple))
}

/// Write every primary-key column on `model` from a complete key value.
pub fn set_primary_key_on_active_model<E>(
    model: &mut E::ActiveModel,
    id: <<E as EntityTrait>::PrimaryKey as PrimaryKeyTrait>::ValueType,
) -> Result<(), PrimaryKeyReadError<E::Column>>
where
    E: EntityTrait,
    E::ActiveModel: ActiveModelTrait<Entity = E>,
    E::Column: ColumnTrait + Copy,
    E::PrimaryKey: PrimaryKeyToColumn<Column = E::Column> + Iterable,
{
    let columns = primary_key_columns::<E>();
    if columns.is_empty() {
        return Err(PrimaryKeyReadError::Missing(ErrMissingPrimaryKey::of::<E>()));
    }
    let values = value_tuple_to_vec(id.into_value_tuple());
    if values.len() != columns.len() {
        return Err(PrimaryKeyReadError::Arity(ErrUnsupportedKeyArity {
            arity: values.len(),
        }));
    }
    for (column, value) in columns.into_iter().zip(values) {
        // `try_set` only fails on a type mismatch, which would be a
        // programming error in the entity definition.
        model
            .try_set(column, value)
            .expect("primary-key column accepts its own ValueType component");
    }
    Ok(())
}

/// Failure when merging an update into an existing model.
#[derive(Debug)]
pub enum MergeUpdateError {
    /// The entity declares no primary-key columns.
    MissingPrimaryKey(ErrMissingPrimaryKey),
    /// SeaORM rejected a `try_set`.
    Db(DbErr),
}

/// Merge `updates` into `existing`, skipping **all** primary-key columns
/// and any field that is `NotSet` on `updates`.
///
/// Only [`sea_orm::ActiveValue::Set`] fields are applied, so a default
/// `ActiveModel` does not clobber existing values with `Unchanged` or
/// `NotSet`.
pub fn merge_update<E>(
    existing: E::Model,
    updates: E::ActiveModel,
) -> Result<E::ActiveModel, MergeUpdateError>
where
    E: EntityTrait,
    E::Model: IntoActiveModel<E::ActiveModel>,
    E::ActiveModel: ActiveModelTrait<Entity = E>,
    E::Column: ColumnTrait + Copy + IdenStatic,
    E::PrimaryKey: PrimaryKeyToColumn<Column = E::Column> + Iterable,
{
    if primary_key_columns::<E>().is_empty() {
        return Err(MergeUpdateError::MissingPrimaryKey(
            ErrMissingPrimaryKey::of::<E>(),
        ));
    }

    let mut model = existing.into_active_model();
    for column in E::Column::iter() {
        if is_primary_key_column::<E>(column) {
            continue;
        }
        if let sea_orm::ActiveValue::Set(value) = updates.get(column) {
            model.try_set(column, value).map_err(MergeUpdateError::Db)?;
        }
    }
    Ok(model)
}

/// Build a [`ValueTuple`] from a vector of values.
///
/// SeaQL 1.x encodes arity 1–3 as dedicated variants and everything else
/// as [`ValueTuple::Many`].
pub fn value_tuple_from_vec(values: Vec<Value>) -> Result<ValueTuple, ErrUnsupportedKeyArity> {
    let arity = values.len();
    let mut iter = values.into_iter();
    let mut next = || iter.next().expect("arity checked");
    match arity {
        0 => Err(ErrUnsupportedKeyArity { arity: 0 }),
        1 => Ok(ValueTuple::One(next())),
        2 => Ok(ValueTuple::Two(next(), next())),
        3 => Ok(ValueTuple::Three(next(), next(), next())),
        _ => {
            let mut rest = vec![next()];
            rest.extend(iter);
            Ok(ValueTuple::Many(rest))
        }
    }
}

/// Flatten a [`ValueTuple`] into a vector, preserving order.
#[must_use]
pub fn value_tuple_to_vec(tuple: ValueTuple) -> Vec<Value> {
    tuple.into_iter().collect()
}

/// Re-export of the default unique-conflict algorithm.
///
/// The canonical location is [`crate::providers::conflict`]. This re-export
/// keeps earlier call sites compiling.
pub use crate::providers::conflict::{is_unique_constraint_violation, message_is_unique};
