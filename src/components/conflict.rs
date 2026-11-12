//! Policy for recognizing a unique-constraint conflict in a [`sea_orm::DbErr`].

use cgp_fork::prelude::*;
use sea_orm::DbErr;

/// Decide whether a SeaORM error is a unique (or primary-key) constraint
/// violation, so [`crate::components::CanGetOrCreateEntity`] can fetch the
/// existing row instead of failing.
#[cgp_component(UniqueConflictDetector)]
pub trait CanDetectUniqueConflict {
    /// Return `true` when `err` is a unique-constraint conflict.
    fn is_unique_conflict(&self, err: &DbErr) -> bool;
}
