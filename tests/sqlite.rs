//! In-memory SQLite coverage for the repository and resource-service layers.

mod support;

use cgp_orm::prelude::*;
use cgp_orm::providers::sql_err_is_unique;
use cgp_orm::util::{merge_update, set_primary_key_on_active_model};
use sea_orm::{ActiveModelTrait, EntityTrait, NotSet, Set, TryIntoModel};
use support::entities::{document, item, membership};
use support::{
    DocumentInput, FailingItemApp, FilteredItemApp, ItemInput, MembershipInput,
    SqlErrOrMessageItemApp, TightPageItemApp, create_table, setup_documents, setup_items,
    setup_memberships,
};

// ---------------------------------------------------------------------------
// i32 primary key: get / insert / update / delete
// ---------------------------------------------------------------------------

#[tokio::test]
async fn item_insert_get_update_delete_by_id() {
    let (app, _db) = setup_items().await;

    let created = app
        .insert(item::ActiveModel {
            id: NotSet,
            name: Set("alpha".into()),
            value: Set(1),
        })
        .await
        .unwrap();
    assert!(created.id > 0);
    assert_eq!(created.name, "alpha");
    assert_eq!(created.value, 1);

    let found = app.get_by_id(created.id).await.unwrap();
    assert_eq!(found.as_ref(), Some(&created));

    let missing = app.get_by_id(created.id + 1_000).await.unwrap();
    assert!(missing.is_none());

    let updated = app
        .update_by_id(item::ActiveModel {
            id: Set(created.id),
            name: Set("alpha".into()),
            value: Set(42),
        })
        .await
        .unwrap()
        .expect("row exists");
    assert_eq!(updated.id, created.id);
    assert_eq!(updated.value, 42);

    let none = app
        .update_by_id(item::ActiveModel {
            id: Set(created.id + 1_000),
            name: Set("ghost".into()),
            value: Set(0),
        })
        .await
        .unwrap();
    assert!(none.is_none());

    assert!(app.delete_by_id(created.id).await.unwrap());
    assert!(!app.delete_by_id(created.id).await.unwrap());
    assert!(app.get_by_id(created.id).await.unwrap().is_none());
}

// ---------------------------------------------------------------------------
// UUID primary key
// ---------------------------------------------------------------------------

#[tokio::test]
async fn uuid_key_insert_get_update_delete() {
    let (app, _db) = setup_documents().await;
    let id = uuid::Uuid::new_v4();

    let created = app
        .insert(document::ActiveModel {
            id: Set(id),
            title: Set("handbook".into()),
            body: Set("v1".into()),
        })
        .await
        .unwrap();
    assert_eq!(created.id, id);

    let found = app.get_by_id(id).await.unwrap().expect("found");
    assert_eq!(found.title, "handbook");

    let updated = app
        .update_by_id(document::ActiveModel {
            id: Set(id),
            title: Set("handbook".into()),
            body: Set("v2".into()),
        })
        .await
        .unwrap()
        .expect("updated");
    assert_eq!(
        updated.id, id,
        "update must not change the UUID primary key"
    );
    assert_eq!(updated.body, "v2");

    assert!(app.delete_by_id(id).await.unwrap());
    assert!(app.get_by_id(id).await.unwrap().is_none());
}

// ---------------------------------------------------------------------------
// Composite primary key
// ---------------------------------------------------------------------------

#[tokio::test]
async fn composite_key_insert_get_update_delete() {
    let (app, _db) = setup_memberships().await;

    let created = app
        .insert(membership::ActiveModel {
            user_id: Set(7),
            group_id: Set(3),
            role: Set("admin".into()),
        })
        .await
        .unwrap();
    assert_eq!(created.user_id, 7);
    assert_eq!(created.group_id, 3);

    // The complete ValueType is a tuple — no From<i32>.
    let found = app
        .get_by_id((7, 3))
        .await
        .unwrap()
        .expect("found by composite id");
    assert_eq!(found.role, "admin");

    assert!(app.get_by_id((7, 99)).await.unwrap().is_none());
    assert!(app.get_by_id((99, 3)).await.unwrap().is_none());

    let updated = app
        .update_by_id(membership::ActiveModel {
            user_id: Set(7),
            group_id: Set(3),
            role: Set("member".into()),
        })
        .await
        .unwrap()
        .expect("updated");
    assert_eq!((updated.user_id, updated.group_id), (7, 3));
    assert_eq!(updated.role, "member");

    // merge_update excludes EVERY primary-key column, even if the incoming
    // active model tries to overwrite them.
    let existing = updated.clone();
    let sneaky = membership::ActiveModel {
        user_id: Set(1_000),
        group_id: Set(2_000),
        role: Set("observer".into()),
    };
    let merged = merge_update::<membership::Entity>(existing, sneaky).unwrap();
    let model = merged.try_into_model().unwrap();
    assert_eq!(
        (model.user_id, model.group_id),
        (7, 3),
        "composite primary key columns must be excluded from merge"
    );
    assert_eq!(model.role, "observer");

    assert!(app.delete_by_id((7, 3)).await.unwrap());
    assert!(!app.delete_by_id((7, 3)).await.unwrap());
}

// ---------------------------------------------------------------------------
// get_or_create under a unique-constraint conflict
// ---------------------------------------------------------------------------

#[tokio::test]
async fn get_or_create_returns_existing_on_unique_conflict() {
    let (app, _db) = setup_items().await;

    let first = app
        .insert(item::ActiveModel {
            id: NotSet,
            name: Set("shared".into()),
            value: Set(1),
        })
        .await
        .unwrap();

    // Plain insert must fail on the unique name — the constraint is real.
    let conflict = app
        .insert(item::ActiveModel {
            id: NotSet,
            name: Set("shared".into()),
            value: Set(99),
        })
        .await;
    assert!(
        conflict.is_err(),
        "insert must not swallow unique conflicts"
    );

    let (second, outcome) = CanGetOrCreateEntity::get_or_create(
        &app,
        item::ActiveModel {
            id: NotSet,
            name: Set("shared".into()),
            value: Set(99),
        },
    )
    .await
    .unwrap();
    assert_eq!(outcome, InsertOutcome::Existing);
    assert_eq!(second.id, first.id);
    assert_eq!(second.value, 1, "existing row is returned, not the input");

    let (third, outcome) = CanGetOrCreateEntity::get_or_create(
        &app,
        item::ActiveModel {
            id: NotSet,
            name: Set("fresh".into()),
            value: Set(3),
        },
    )
    .await
    .unwrap();
    assert_eq!(outcome, InsertOutcome::Inserted);
    assert_eq!(
        third.id,
        first.id + 1,
        "a repeat get_or_create must not consume an id"
    );
}

#[tokio::test]
async fn get_or_create_composite_key_conflict() {
    let (app, _db) = setup_memberships().await;

    let (first, outcome) = CanGetOrCreateEntity::get_or_create(
        &app,
        membership::ActiveModel {
            user_id: Set(1),
            group_id: Set(2),
            role: Set("admin".into()),
        },
    )
    .await
    .unwrap();
    assert_eq!(outcome, InsertOutcome::Inserted);

    let (second, outcome) = CanGetOrCreateEntity::get_or_create(
        &app,
        membership::ActiveModel {
            user_id: Set(1),
            group_id: Set(2),
            role: Set("ignored".into()),
        },
    )
    .await
    .unwrap();
    assert_eq!(outcome, InsertOutcome::Existing);
    assert_eq!(second.role, "admin");
    assert_eq!(
        (second.user_id, second.group_id),
        (first.user_id, first.group_id)
    );
}

// ---------------------------------------------------------------------------
// upsert
// ---------------------------------------------------------------------------

#[tokio::test]
async fn upsert_inserts_then_updates() {
    let (app, _db) = setup_items().await;

    let (created, outcome) = app
        .upsert(item::ActiveModel {
            id: NotSet,
            name: Set("k".into()),
            value: Set(1),
        })
        .await
        .unwrap();
    assert_eq!(outcome, UpsertOutcome::Inserted);
    assert_eq!(created.value, 1);

    let (updated, outcome) = app
        .upsert(item::ActiveModel {
            id: NotSet,
            name: Set("k".into()),
            value: Set(8),
        })
        .await
        .unwrap();
    assert_eq!(outcome, UpsertOutcome::Updated);
    assert_eq!(updated.id, created.id);
    assert_eq!(updated.value, 8);

    let page = app.paginate(PageRequest::new(1, 10)).await.unwrap();
    assert_eq!(page.total, Some(1), "upsert must not insert a second row");

    let (other, outcome) = app
        .upsert(item::ActiveModel {
            id: NotSet,
            name: Set("k2".into()),
            value: Set(3),
        })
        .await
        .unwrap();
    assert_eq!(outcome, UpsertOutcome::Inserted);
    assert_eq!(
        other.id,
        created.id + 1,
        "updating an existing identity must not consume an id"
    );
}

// ---------------------------------------------------------------------------
// Pagination: deterministic order + max-size clamping, never size 5
// ---------------------------------------------------------------------------

#[tokio::test]
async fn pagination_is_deterministic_and_clamps_size() {
    let (app, db) = setup_items().await;

    for i in 0..10 {
        app.insert(item::ActiveModel {
            id: NotSet,
            name: Set(format!("n{i}")),
            value: Set(i),
        })
        .await
        .unwrap();
    }

    let page = app.paginate(PageRequest::new(1, 3)).await.unwrap();
    assert_eq!(page.size, 3);
    assert_eq!(page.page, 1);
    assert_eq!(page.total, Some(10));
    assert_eq!(page.items.len(), 3);
    let ids: Vec<i32> = page.items.iter().map(|m| m.id).collect();
    let mut sorted = ids.clone();
    sorted.sort();
    assert_eq!(
        ids, sorted,
        "primary-key ordering must be deterministic and ascending"
    );
    assert_eq!(ids, vec![1, 2, 3]);

    let page2 = app.paginate(PageRequest::new(2, 3)).await.unwrap();
    assert_eq!(
        page2.items.iter().map(|m| m.id).collect::<Vec<_>>(),
        vec![4, 5, 6]
    );

    // Tight policy: default 10, max 3. A request of 999 must clamp to 3.
    // It must never silently become 5.
    let tight = TightPageItemApp::new(db);
    let clamped = tight.paginate(PageRequest::new(1, 999)).await.unwrap();
    assert_eq!(clamped.size, 3);
    assert_ne!(clamped.size, 5);
    assert_eq!(clamped.items.len(), 3);
    assert_eq!(clamped.total, Some(10));

    let defaulted = tight.paginate(PageRequest::first()).await.unwrap();
    assert_eq!(
        defaulted.size, 3,
        "default 10 is itself clamped to the maximum of 3"
    );
    assert_ne!(defaulted.size, 5);
}

// ---------------------------------------------------------------------------
// Rollback preserves the original error
// ---------------------------------------------------------------------------

#[tokio::test]
async fn rollback_preserves_the_original_hook_error() {
    let db = support::connect().await;
    create_table(&db, item::Entity).await;
    let app = FailingItemApp::new(db.clone());

    let err = app
        .insert(item::ActiveModel {
            id: NotSet,
            name: Set("doomed".into()),
            value: Set(1),
        })
        .await
        .expect_err("hook must fail the insert");

    let message = err.to_string();
    assert!(
        message.contains("hook failed"),
        "original hook error must be preserved, got: {message}"
    );
    assert!(
        !message.to_ascii_lowercase().contains("rollback"),
        "rollback must not replace the original error, got: {message}"
    );

    let count = item::Entity::find().all(&db).await.unwrap().len();
    assert_eq!(count, 0, "failed write must be rolled back");
}

#[tokio::test]
async fn get_or_create_existing_skips_the_write_hook() {
    let (app, db) = setup_items().await;
    let created = app
        .insert(item::ActiveModel {
            id: NotSet,
            name: Set("shared".into()),
            value: Set(1),
        })
        .await
        .unwrap();

    let failing = FailingItemApp::new(db);
    let (existing, outcome) = CanGetOrCreateEntity::get_or_create(
        &failing,
        item::ActiveModel {
            id: NotSet,
            name: Set("shared".into()),
            value: Set(99),
        },
    )
    .await
    .expect("an existing row must not run the write hook");
    assert_eq!(outcome, InsertOutcome::Existing);
    assert_eq!(existing.id, created.id);
    assert_eq!(existing.value, 1);
}

// ---------------------------------------------------------------------------
// Resource service layer
// ---------------------------------------------------------------------------

#[tokio::test]
async fn resource_service_create_get_update_list_delete() {
    let (app, _db) = setup_items().await;

    let (created, outcome) = app
        .create(ItemInput {
            name: "svc".into(),
            value: 1,
        })
        .await
        .unwrap();
    assert_eq!(outcome, InsertOutcome::Inserted);
    assert_eq!(created.name, "svc");

    let got = app.get(created.id).await.unwrap().expect("get");
    assert_eq!(got, created);

    let updated = app
        .update(
            created.id,
            ItemInput {
                name: "svc".into(),
                value: 9,
            },
        )
        .await
        .unwrap()
        .expect("update");
    assert_eq!(updated.value, 9);

    let page = app.list(PageRequest::new(1, 10)).await.unwrap();
    assert_eq!(page.total, Some(1));
    assert_eq!(page.items[0].value, 9);

    assert!(app.delete(created.id).await.unwrap());
    assert!(app.get(created.id).await.unwrap().is_none());
}

#[tokio::test]
async fn resource_service_create_returns_existing() {
    let (app, _db) = setup_items().await;
    let input = ItemInput {
        name: "once".into(),
        value: 1,
    };
    let (first, outcome) = app.create(input).await.unwrap();
    assert_eq!(outcome, InsertOutcome::Inserted);

    let (second, outcome) = app
        .create(ItemInput {
            name: "once".into(),
            value: 99,
        })
        .await
        .unwrap();
    assert_eq!(outcome, InsertOutcome::Existing);
    assert_eq!(second.id, first.id);
    assert_eq!(second.value, 1);
}

#[tokio::test]
async fn resource_service_uuid_and_composite() {
    let (docs, _) = setup_documents().await;
    let id = uuid::Uuid::new_v4();
    let (created, outcome) = docs
        .create(DocumentInput {
            id,
            title: "t".into(),
            body: "b".into(),
        })
        .await
        .unwrap();
    assert_eq!(outcome, InsertOutcome::Inserted);
    assert_eq!(created.id, id);
    assert_eq!(docs.get(id).await.unwrap().unwrap().title, "t");

    let (mems, _) = setup_memberships().await;
    let (created, outcome) = mems
        .create(MembershipInput {
            user_id: 4,
            group_id: 5,
            role: "r".into(),
        })
        .await
        .unwrap();
    assert_eq!(outcome, InsertOutcome::Inserted);
    assert_eq!((created.user_id, created.group_id), (4, 5));
    let got = mems.get((4, 5)).await.unwrap().expect("composite get");
    assert_eq!(got.role, "r");
}

// ---------------------------------------------------------------------------
// Structured unique-conflict classification (sql_err)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn unique_violation_is_classified_by_sql_err() {
    let (app, db) = setup_items().await;

    app.insert(item::ActiveModel {
        id: NotSet,
        name: Set("shared".into()),
        value: Set(1),
    })
    .await
    .unwrap();

    // A raw SeaORM insert yields the DbErr that StandardUniqueConflict
    // classifies first via DbErr::sql_err(). Dev-dependencies enable
    // sqlx-sqlite, so this must not fall through to the message matcher.
    let raw = item::ActiveModel {
        id: NotSet,
        name: Set("shared".into()),
        value: Set(99),
    }
    .insert(&db)
    .await
    .expect_err("second insert must hit the unique name constraint");

    assert!(
        sql_err_is_unique(&raw),
        "a real SQLite unique failure must make sql_err_is_unique return true; \
         got {raw:?}. A SeaORM regression must not silently fall through to \
         the message fallback on the path this suite hits."
    );

    // The repository `insert` path raises the same DbErr.
    let wrapped = app
        .insert(item::ActiveModel {
            id: NotSet,
            name: Set("shared".into()),
            value: Set(99),
        })
        .await
        .expect_err("insert must not swallow unique conflicts");
    let db_err = wrapped
        .downcast_ref::<sea_orm::DbErr>()
        .expect("insert raises a SeaORM DbErr through the error map");
    assert!(
        sql_err_is_unique(db_err),
        "the repository insert path must surface the same structured classifier; \
         got {db_err:?}"
    );
}

// ---------------------------------------------------------------------------
// OrUniqueConflict wiring (SqlErr + message, not UseDelegate)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn or_unique_conflict_sql_err_and_message_handles_real_conflict() {
    let db = support::connect().await;
    create_table(&db, item::Entity).await;
    let app = SqlErrOrMessageItemApp::new(db);

    let (first, outcome) = CanGetOrCreateEntity::get_or_create(
        &app,
        item::ActiveModel {
            id: NotSet,
            name: Set("shared".into()),
            value: Set(1),
        },
    )
    .await
    .unwrap();
    assert_eq!(outcome, InsertOutcome::Inserted);

    let (second, outcome) = CanGetOrCreateEntity::get_or_create(
        &app,
        item::ActiveModel {
            id: NotSet,
            name: Set("shared".into()),
            value: Set(99),
        },
    )
    .await
    .unwrap();
    assert_eq!(outcome, InsertOutcome::Existing);
    assert_eq!(second.id, first.id);
    assert_eq!(second.value, 1);
}

// ---------------------------------------------------------------------------
// Entity filter (tenant-like) and optional COUNT(*)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn default_entity_filter_returns_all_rows() {
    let (app, _db) = setup_items().await;
    for i in 0..4 {
        app.insert(item::ActiveModel {
            id: NotSet,
            name: Set(format!("n{i}")),
            value: Set(i),
        })
        .await
        .unwrap();
    }

    let page = app.paginate(PageRequest::new(1, 10)).await.unwrap();
    assert_eq!(page.total, Some(4), "NoEntityFilter must not hide rows");
    assert_eq!(page.items.len(), 4);
}

#[tokio::test]
async fn custom_entity_filter_restricts_paginate_and_identity() {
    let (app, db) = setup_items().await;

    app.insert(item::ActiveModel {
        id: NotSet,
        name: Set("tenant-1".into()),
        value: Set(1),
    })
    .await
    .unwrap();
    app.insert(item::ActiveModel {
        id: NotSet,
        name: Set("tenant-2".into()),
        value: Set(2),
    })
    .await
    .unwrap();
    app.insert(item::ActiveModel {
        id: NotSet,
        name: Set("tenant-1-b".into()),
        value: Set(1),
    })
    .await
    .unwrap();

    // Unfiltered context still sees every row.
    let all = app.paginate(PageRequest::new(1, 10)).await.unwrap();
    assert_eq!(all.total, Some(3));
    assert_eq!(all.items.len(), 3);

    // `value = 1` stands in for a tenant id (or a soft-delete predicate).
    let filtered = FilteredItemApp::new(db);
    let page = filtered.paginate(PageRequest::new(1, 10)).await.unwrap();
    assert_eq!(
        page.total,
        Some(2),
        "COUNT(*) must use the same entity filter as the page query"
    );
    let names: Vec<&str> = page.items.iter().map(|m| m.name.as_str()).collect();
    assert_eq!(names, vec!["tenant-1", "tenant-1-b"]);
    assert!(page.items.iter().all(|m| m.value == 1));

    // Identity find consults the same filter.
    let hit = filtered
        .find_by_identity(&item::ActiveModel {
            id: NotSet,
            name: Set("tenant-1".into()),
            value: NotSet,
        })
        .await
        .unwrap();
    assert_eq!(hit.unwrap().name, "tenant-1");

    let missed = filtered
        .find_by_identity(&item::ActiveModel {
            id: NotSet,
            name: Set("tenant-2".into()),
            value: NotSet,
        })
        .await
        .unwrap();
    assert!(
        missed.is_none(),
        "a row excluded by the entity filter must not be found by identity"
    );
}

#[tokio::test]
async fn paginate_can_skip_the_total_count() {
    let (app, _db) = setup_items().await;
    for i in 0..5 {
        app.insert(item::ActiveModel {
            id: NotSet,
            name: Set(format!("n{i}")),
            value: Set(i),
        })
        .await
        .unwrap();
    }

    let counted = app.paginate(PageRequest::new(1, 3)).await.unwrap();
    assert_eq!(counted.total, Some(5));
    assert_eq!(counted.items.len(), 3);
    assert_eq!(counted.has_next(), Some(true));
    assert_eq!(counted.total_pages(), Some(2));

    let skipped = app
        .paginate(PageRequest::new(1, 3).without_total())
        .await
        .unwrap();
    assert_eq!(skipped.total, None, "COUNT(*) must be skipped");
    assert_eq!(skipped.items.len(), 3);
    assert_eq!(skipped.page, 1);
    assert_eq!(skipped.size, 3);
    assert_eq!(skipped.has_next(), None);
    assert_eq!(skipped.total_pages(), None);
    assert!(
        skipped.maybe_has_next(),
        "a full page without a total is a hint that another page may exist"
    );

    let last = app
        .paginate(PageRequest::new(2, 3).without_total())
        .await
        .unwrap();
    assert_eq!(last.total, None);
    assert_eq!(last.items.len(), 2);
    assert!(!last.maybe_has_next());
}

#[test]
fn set_primary_key_writes_every_column_without_panicking() {
    let mut model = item::ActiveModel {
        id: NotSet,
        name: NotSet,
        value: NotSet,
    };
    set_primary_key_on_active_model::<item::Entity>(&mut model, 7).unwrap();
    assert_eq!(model.id, Set(7));

    let mut membership = membership::ActiveModel {
        user_id: NotSet,
        group_id: NotSet,
        role: NotSet,
    };
    set_primary_key_on_active_model::<membership::Entity>(&mut membership, (4, 5)).unwrap();
    assert_eq!(membership.user_id, Set(4));
    assert_eq!(membership.group_id, Set(5));
}
