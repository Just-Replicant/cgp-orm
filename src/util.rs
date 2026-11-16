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

use crate::error::{
    ErrMissingPrimaryKey, ErrPrimaryKeySetFailed, ErrPrimaryKeyUnset, ErrUnsupportedKeyArity,
};

/// Failure when reading a complete primary key from an active model.
#[derive(Debug)]
pub enum PrimaryKeyReadError<Column> {
    /// The entity declares no primary-key columns.
    Missing(ErrMissingPrimaryKey),
    /// At least one primary-key column is `NotSet`.
    Unset(ErrPrimaryKeyUnset<Column>),
}

/// Collect every primary-key column of `E`.
///
/// The result may be empty only for a misconfigured entity; callers treat
/// that as [`ErrMissingPrimaryKey`]. The crate's own helpers walk
/// [`Iterable::iter`] in place instead of calling this, so they do not
/// allocate; this collecting form is kept for callers that want an owned
/// list.
#[must_use]
pub fn primary_key_columns<E>() -> Vec<E::Column>
where
    E: EntityTrait,
    E::PrimaryKey: PrimaryKeyToColumn<Column = E::Column> + Iterable,
{
    primary_key_column_iter::<E>().collect()
}

/// Iterate the primary-key columns of `E` in declaration order, without
/// allocating.
fn primary_key_column_iter<E>() -> impl Iterator<Item = E::Column>
where
    E: EntityTrait,
    E::PrimaryKey: PrimaryKeyToColumn<Column = E::Column> + Iterable,
{
    E::PrimaryKey::iter().map(PrimaryKeyToColumn::into_column)
}

/// Whether `E` declares at least one primary-key column.
fn has_primary_key<E>() -> bool
where
    E: EntityTrait,
    E::PrimaryKey: Iterable,
{
    E::PrimaryKey::iter().next().is_some()
}

/// Whether `column` is one of the entity's primary-key columns.
///
/// Comparison is by SeaORM identifier string so it does not require
/// `PartialEq` on the column enum and does not special-case the first key.
/// The key columns are walked in place; nothing is collected.
#[must_use]
pub fn is_primary_key_column<E>(column: E::Column) -> bool
where
    E: EntityTrait,
    E::PrimaryKey: PrimaryKeyToColumn<Column = E::Column> + Iterable,
    E::Column: IdenStatic,
{
    let name = column.as_str();
    primary_key_column_iter::<E>().any(|pk| pk.as_str() == name)
}

/// Read the complete primary-key [`ValueType`](PrimaryKeyTrait::ValueType)
/// from an active model, including every key column.
///
/// Keys of arity 1–3 are built directly as [`ValueTuple::One`],
/// [`ValueTuple::Two`], or [`ValueTuple::Three`] without an intermediate
/// `Vec`. Only a key of four or more columns allocates, for
/// [`ValueTuple::Many`].
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
    let read = |column: E::Column| {
        model
            .get(column)
            .into_value()
            .ok_or(PrimaryKeyReadError::Unset(ErrPrimaryKeyUnset { column }))
    };

    let mut columns = primary_key_column_iter::<E>();
    let Some(first) = columns.next() else {
        return Err(PrimaryKeyReadError::Missing(ErrMissingPrimaryKey::of::<E>()));
    };
    let a = read(first)?;
    let Some(second) = columns.next() else {
        return Ok(FromValueTuple::from_value_tuple(ValueTuple::One(a)));
    };
    let b = read(second)?;
    let Some(third) = columns.next() else {
        return Ok(FromValueTuple::from_value_tuple(ValueTuple::Two(a, b)));
    };
    let c = read(third)?;
    let Some(fourth) = columns.next() else {
        return Ok(FromValueTuple::from_value_tuple(ValueTuple::Three(a, b, c)));
    };

    let mut values = Vec::with_capacity(4 + columns.size_hint().0);
    values.extend([a, b, c, read(fourth)?]);
    for column in columns {
        values.push(read(column)?);
    }
    Ok(FromValueTuple::from_value_tuple(ValueTuple::Many(values)))
}

/// Failure when writing a complete primary key onto an active model.
#[derive(Debug)]
pub enum PrimaryKeyWriteError {
    /// The entity declares no primary-key columns.
    Missing(ErrMissingPrimaryKey),
    /// The key value's arity does not match the entity's key columns.
    Arity(ErrUnsupportedKeyArity),
    /// [`ActiveModelTrait::try_set`] rejected a primary-key value.
    ///
    /// This is typically a type mismatch in the entity definition. It is
    /// a typed error so a request path can raise it through
    /// `CanRaiseError` instead of panicking.
    SetFailed(ErrPrimaryKeySetFailed),
}

/// Write every primary-key column on `model` from a complete key value.
///
/// A key of arity 1–3 is destructured from [`ValueTuple::One`],
/// [`ValueTuple::Two`], or [`ValueTuple::Three`] in place; a
/// [`ValueTuple::Many`] key consumes the `Vec` it already owns. No
/// intermediate `Vec` is built. The arity is checked against the entity
/// before any column is written, so a mismatch leaves `model` untouched.
///
/// A `try_set` failure is returned as [`PrimaryKeyWriteError::SetFailed`]
/// rather than panicking, so a bad entity definition is a typed error on
/// the request path. Callers raise it through `CanRaiseError`.
pub fn set_primary_key_on_active_model<E>(
    model: &mut E::ActiveModel,
    id: <<E as EntityTrait>::PrimaryKey as PrimaryKeyTrait>::ValueType,
) -> Result<(), PrimaryKeyWriteError>
where
    E: EntityTrait,
    E::ActiveModel: ActiveModelTrait<Entity = E>,
    E::Column: ColumnTrait + Copy,
    E::PrimaryKey: PrimaryKeyToColumn<Column = E::Column> + Iterable,
{
    if !has_primary_key::<E>() {
        return Err(PrimaryKeyWriteError::Missing(
            ErrMissingPrimaryKey::of::<E>(),
        ));
    }
    match id.into_value_tuple() {
        ValueTuple::One(a) => assign_primary_key::<E, _>(model, [a]),
        ValueTuple::Two(a, b) => assign_primary_key::<E, _>(model, [a, b]),
        ValueTuple::Three(a, b, c) => assign_primary_key::<E, _>(model, [a, b, c]),
        ValueTuple::Many(values) => assign_primary_key::<E, _>(model, values),
    }
}

/// Zip the entity's primary-key columns with `values` and `try_set` each.
///
/// `values` must have exactly as many items as the entity has key columns.
fn assign_primary_key<E, V>(
    model: &mut E::ActiveModel,
    values: V,
) -> Result<(), PrimaryKeyWriteError>
where
    E: EntityTrait,
    E::ActiveModel: ActiveModelTrait<Entity = E>,
    E::Column: ColumnTrait + Copy,
    E::PrimaryKey: PrimaryKeyToColumn<Column = E::Column> + Iterable,
    V: IntoIterator<Item = Value>,
    V::IntoIter: ExactSizeIterator,
{
    let values = values.into_iter();
    let arity = values.len();
    if arity != E::PrimaryKey::iter().count() {
        return Err(PrimaryKeyWriteError::Arity(ErrUnsupportedKeyArity {
            arity,
        }));
    }
    for (column, value) in primary_key_column_iter::<E>().zip(values) {
        model.try_set(column, value).map_err(|err| {
            PrimaryKeyWriteError::SetFailed(ErrPrimaryKeySetFailed::of::<E>(&err))
        })?;
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
    if !has_primary_key::<E>() {
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

/// Re-export of the default unique-conflict algorithm.
///
/// The canonical location is [`crate::providers::conflict`]. This re-export
/// keeps earlier call sites compiling.
pub use crate::providers::conflict::{is_unique_constraint_violation, message_is_unique};

#[cfg(test)]
mod tests {
    use sea_orm::{NotSet, Set};

    use super::*;

    mod triple {
        use sea_orm::entity::prelude::*;

        #[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
        #[sea_orm(table_name = "triples")]
        pub struct Model {
            #[sea_orm(primary_key, auto_increment = false)]
            pub a: i32,
            #[sea_orm(primary_key, auto_increment = false)]
            pub b: i32,
            #[sea_orm(primary_key, auto_increment = false)]
            pub c: i32,
            pub note: String,
        }

        #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
        pub enum Relation {}

        impl ActiveModelBehavior for ActiveModel {}
    }

    mod quad {
        use sea_orm::entity::prelude::*;

        #[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
        #[sea_orm(table_name = "quads")]
        pub struct Model {
            #[sea_orm(primary_key, auto_increment = false)]
            pub a: i32,
            #[sea_orm(primary_key, auto_increment = false)]
            pub b: i32,
            #[sea_orm(primary_key, auto_increment = false)]
            pub c: i32,
            #[sea_orm(primary_key, auto_increment = false)]
            pub d: i32,
            pub note: String,
        }

        #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
        pub enum Relation {}

        impl ActiveModelBehavior for ActiveModel {}
    }

    #[test]
    fn three_column_key_round_trips() {
        let mut model = triple::ActiveModel {
            a: NotSet,
            b: NotSet,
            c: NotSet,
            note: Set("x".into()),
        };
        set_primary_key_on_active_model::<triple::Entity>(&mut model, (1, 2, 3)).unwrap();
        assert_eq!(
            (model.a.clone(), model.b.clone(), model.c.clone()),
            (Set(1), Set(2), Set(3))
        );
        assert_eq!(
            primary_key_from_active_model::<triple::Entity>(&model).unwrap(),
            (1, 2, 3)
        );
    }

    #[test]
    fn four_column_key_round_trips_through_many() {
        let mut model = quad::ActiveModel {
            a: NotSet,
            b: NotSet,
            c: NotSet,
            d: NotSet,
            note: Set("x".into()),
        };
        set_primary_key_on_active_model::<quad::Entity>(&mut model, (1, 2, 3, 4)).unwrap();
        assert_eq!(model.d, Set(4));
        assert_eq!(
            primary_key_from_active_model::<quad::Entity>(&model).unwrap(),
            (1, 2, 3, 4)
        );
    }

    #[test]
    fn unset_key_column_is_reported_by_name() {
        let model = quad::ActiveModel {
            a: Set(1),
            b: Set(2),
            c: Set(3),
            d: NotSet,
            note: NotSet,
        };
        match primary_key_from_active_model::<quad::Entity>(&model) {
            Err(PrimaryKeyReadError::Unset(err)) => {
                assert_eq!(err.column.as_str(), quad::Column::D.as_str());
            }
            other => panic!("expected an unset key column, got {other:?}"),
        }
    }

    #[test]
    fn merge_update_skips_every_key_column() {
        let existing = quad::Model {
            a: 1,
            b: 2,
            c: 3,
            d: 4,
            note: "old".into(),
        };
        let updates = quad::ActiveModel {
            a: Set(9),
            b: Set(9),
            c: Set(9),
            d: Set(9),
            note: Set("new".into()),
        };
        let merged = merge_update::<quad::Entity>(existing, updates).unwrap();
        assert_eq!(
            (
                merged.a.as_ref(),
                merged.b.as_ref(),
                merged.c.as_ref(),
                merged.d.as_ref()
            ),
            (&1, &2, &3, &4)
        );
        assert_eq!(merged.note, Set("new".to_owned()));
        assert!(is_primary_key_column::<quad::Entity>(quad::Column::D));
        assert!(!is_primary_key_column::<quad::Entity>(quad::Column::Note));
    }
}
