use phenix_core::{
    InitialObservation, Key, ObservableError, ObservableRegistration, ObservableStore,
    ObservationDelivery, ObservationMode, ObservationScope, ObservationSpec, PhenixValue, PluginId,
    SnapshotPolicy, Type, ValueAddress, ValueChange, ValueId, ValuePath, ValuePathSegment,
};
use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
};

#[derive(Clone, Debug, PartialEq)]
struct SeenDelivery {
    commit: Option<u64>,
    from_version: u64,
    version: u64,
    changes: Vec<ValueChange>,
    full: Option<PhenixValue>,
}

fn id(value: &str) -> ValueId {
    ValueId::parse(value).unwrap()
}

fn field(value: &str) -> Key {
    Key::parse(value).unwrap()
}

fn path(segments: impl IntoIterator<Item = ValuePathSegment>) -> ValuePath {
    ValuePath::new(segments)
}

fn table(fields: impl IntoIterator<Item = (&'static str, PhenixValue)>) -> PhenixValue {
    PhenixValue::Table(
        fields
            .into_iter()
            .map(|(key, value)| (field(key), value))
            .collect(),
    )
}

fn nested_schema() -> Type {
    Type::Table(BTreeMap::from([(
        field("nested"),
        Type::Table(BTreeMap::from([(field("leaf"), Type::U64)])),
    )]))
}

fn nested_value(leaf: u64) -> PhenixValue {
    table([("nested", table([("leaf", PhenixValue::U64(leaf))]))])
}

fn register(store: &ObservableStore, value: &ValueId, schema: Type, initial: PhenixValue) {
    store
        .register(ObservableRegistration {
            id: value.clone(),
            owner: PluginId::parse("observable-semantics-test").unwrap(),
            schema,
            snapshot_policy: SnapshotPolicy::CopyOnChange,
            initial,
        })
        .unwrap();
}

fn record(
    deliveries: Arc<Mutex<Vec<SeenDelivery>>>,
) -> Arc<impl Fn(ObservationDelivery<'_>) + Send + Sync> {
    Arc::new(move |delivery: ObservationDelivery<'_>| {
        deliveries.lock().unwrap().push(SeenDelivery {
            commit: delivery.commit.map(|commit| commit.get()),
            from_version: delivery.from_version.get(),
            version: delivery.version.get(),
            changes: delivery.changes.to_vec(),
            full: delivery.full_value().unwrap().cloned(),
        });
    })
}

#[test]
fn matching_coalescing_noop_and_unsubscribe_are_semantic() {
    let store = ObservableStore::default();
    let value = id("semantics.tree@1");
    register(&store, &value, nested_schema(), nested_value(1));

    let nested = path([ValuePathSegment::Field(field("nested"))]);
    let leaf = path([
        ValuePathSegment::Field(field("nested")),
        ValuePathSegment::Field(field("leaf")),
    ]);
    let exact_seen = Arc::new(Mutex::new(Vec::new()));
    let recursive_seen = Arc::new(Mutex::new(Vec::new()));

    let exact = store
        .subscribe(
            ObservationSpec {
                address: ValueAddress {
                    value: value.clone(),
                    path: nested.clone(),
                },
                scope: ObservationScope::Exact,
                mode: ObservationMode::Diff,
                initial: InitialObservation::None,
            },
            record(Arc::clone(&exact_seen)),
        )
        .unwrap();
    let recursive = store
        .subscribe(
            ObservationSpec {
                address: ValueAddress {
                    value: value.clone(),
                    path: nested.clone(),
                },
                scope: ObservationScope::Recursive,
                mode: ObservationMode::Diff,
                initial: InitialObservation::None,
            },
            record(Arc::clone(&recursive_seen)),
        )
        .unwrap();

    store
        .transaction([&value], |tx| {
            tx.replace(&value, leaf.clone(), PhenixValue::U64(2))
        })
        .unwrap();
    assert!(exact_seen.lock().unwrap().is_empty());
    assert_eq!(recursive_seen.lock().unwrap().len(), 1);

    store
        .transaction([&value], |tx| {
            tx.replace(
                &value,
                nested.clone(),
                table([("leaf", PhenixValue::U64(3))]),
            )
        })
        .unwrap();
    assert_eq!(exact_seen.lock().unwrap().len(), 1);
    assert_eq!(recursive_seen.lock().unwrap().len(), 2);

    let version_before_coalesced = store.metadata(&value).unwrap().version.get();
    store
        .transaction([&value], |tx| {
            tx.replace(&value, leaf.clone(), PhenixValue::U64(4))?;
            tx.replace(&value, leaf.clone(), PhenixValue::U64(5))?;
            Ok(())
        })
        .unwrap();
    let recursive = recursive_seen.lock().unwrap();
    assert_eq!(recursive.len(), 3);
    assert_eq!(recursive.last().unwrap().version, version_before_coalesced + 1);
    assert_eq!(recursive.last().unwrap().changes.len(), 1);
    assert_eq!(
        recursive.last().unwrap().changes,
        vec![ValueChange::Replace {
            path: path([ValuePathSegment::Field(field("leaf"))]),
            value: PhenixValue::U64(5),
        }]
    );
    drop(recursive);

    let version_before_noop = store.metadata(&value).unwrap().version.get();
    let deliveries_before_noop = recursive_seen.lock().unwrap().len();
    store
        .transaction([&value], |tx| {
            tx.replace(&value, leaf.clone(), PhenixValue::U64(6))?;
            tx.replace(&value, leaf.clone(), PhenixValue::U64(5))?;
            Ok(())
        })
        .unwrap();
    assert_eq!(store.metadata(&value).unwrap().version.get(), version_before_noop);
    assert_eq!(recursive_seen.lock().unwrap().len(), deliveries_before_noop);

    assert!(store.unsubscribe(&recursive).unwrap());
    assert!(!store.unsubscribe(&recursive).unwrap());
    store
        .transaction([&value], |tx| {
            tx.replace(&value, leaf, PhenixValue::U64(7))
        })
        .unwrap();
    assert_eq!(recursive_seen.lock().unwrap().len(), deliveries_before_noop);
    assert!(store.unsubscribe(&exact).unwrap());
}

#[test]
fn multi_root_commit_is_atomic_and_failed_transaction_rolls_back_versions() {
    let store = ObservableStore::default();
    let left = id("semantics.left@1");
    let right = id("semantics.right@1");
    register(&store, &left, Type::U64, PhenixValue::U64(1));
    register(&store, &right, Type::U64, PhenixValue::U64(10));

    let seen = Arc::new(Mutex::new(Vec::<(String, SeenDelivery, u64, u64)>::new()));
    for observed in [&left, &right] {
        let store = store.clone();
        let left = left.clone();
        let right = right.clone();
        let observed_name = observed.as_str().to_owned();
        let seen = Arc::clone(&seen);
        store
            .subscribe(
                ObservationSpec {
                    address: ValueAddress {
                        value: observed.clone(),
                        path: ValuePath::root(),
                    },
                    scope: ObservationScope::Exact,
                    mode: ObservationMode::Full,
                    initial: InitialObservation::None,
                },
                Arc::new(move |delivery: ObservationDelivery<'_>| {
                    // Re-entering get() proves callbacks run after the mutation lock is released.
                    let left_value = match store
                        .get(&ValueAddress {
                            value: left.clone(),
                            path: ValuePath::root(),
                        })
                        .unwrap()
                        .1
                    {
                        PhenixValue::U64(value) => value,
                        value => panic!("unexpected left value {value:?}"),
                    };
                    let right_value = match store
                        .get(&ValueAddress {
                            value: right.clone(),
                            path: ValuePath::root(),
                        })
                        .unwrap()
                        .1
                    {
                        PhenixValue::U64(value) => value,
                        value => panic!("unexpected right value {value:?}"),
                    };
                    seen.lock().unwrap().push((
                        observed_name.clone(),
                        SeenDelivery {
                            commit: delivery.commit.map(|commit| commit.get()),
                            from_version: delivery.from_version.get(),
                            version: delivery.version.get(),
                            changes: delivery.changes.to_vec(),
                            full: delivery.full_value().unwrap().cloned(),
                        },
                        left_value,
                        right_value,
                    ));
                }),
            )
            .unwrap();
    }

    store
        .transaction([&left, &right], |tx| {
            tx.replace(&left, ValuePath::root(), PhenixValue::U64(2))?;
            tx.replace(&right, ValuePath::root(), PhenixValue::U64(20))?;
            Ok(())
        })
        .unwrap();

    let committed = seen.lock().unwrap().clone();
    assert_eq!(committed.len(), 2);
    assert_eq!(committed[0].1.commit, committed[1].1.commit);
    assert!(committed[0].1.commit.is_some());
    assert!(committed.iter().all(|(_, delivery, left_value, right_value)| {
        delivery.from_version == 0
            && delivery.version == 1
            && *left_value == 2
            && *right_value == 20
    }));
    drop(committed);

    let deliveries_before_abort = seen.lock().unwrap().len();
    let left_version = store.metadata(&left).unwrap().version.get();
    let right_version = store.metadata(&right).unwrap().version.get();
    let result: Result<(), ObservableError> = store.transaction([&left, &right], |tx| {
        tx.replace(&left, ValuePath::root(), PhenixValue::U64(3))?;
        tx.replace(&right, ValuePath::root(), PhenixValue::U64(30))?;
        Err(ObservableError::TransactionConflict {
            value: left.clone(),
            message: "abort regression".to_owned(),
        })
    });
    assert!(matches!(result, Err(ObservableError::TransactionConflict { .. })));
    assert_eq!(store.metadata(&left).unwrap().version.get(), left_version);
    assert_eq!(store.metadata(&right).unwrap().version.get(), right_version);
    assert_eq!(seen.lock().unwrap().len(), deliveries_before_abort);
    assert_eq!(
        store
            .get(&ValueAddress {
                value: left.clone(),
                path: ValuePath::root(),
            })
            .unwrap()
            .1,
        PhenixValue::U64(2)
    );
    assert_eq!(
        store
            .get(&ValueAddress {
                value: right,
                path: ValuePath::root(),
            })
            .unwrap()
            .1,
        PhenixValue::U64(20)
    );
}

#[test]
fn initial_full_is_versioned_before_later_commits() {
    let store = ObservableStore::default();
    let value = id("semantics.initial@1");
    register(&store, &value, Type::U64, PhenixValue::U64(7));
    let seen = Arc::new(Mutex::new(Vec::new()));

    store
        .subscribe(
            ObservationSpec {
                address: ValueAddress {
                    value: value.clone(),
                    path: ValuePath::root(),
                },
                scope: ObservationScope::Exact,
                mode: ObservationMode::Full,
                initial: InitialObservation::Full,
            },
            record(Arc::clone(&seen)),
        )
        .unwrap();

    store
        .transaction([&value], |tx| {
            tx.replace(&value, ValuePath::root(), PhenixValue::U64(8))
        })
        .unwrap();

    let seen = seen.lock().unwrap();
    assert_eq!(seen.len(), 2);
    assert_eq!(seen[0].commit, None);
    assert_eq!(seen[0].from_version, 0);
    assert_eq!(seen[0].version, 0);
    assert_eq!(seen[0].full, Some(PhenixValue::U64(7)));
    assert!(seen[1].commit.is_some());
    assert_eq!(seen[1].from_version, 0);
    assert_eq!(seen[1].version, 1);
    assert_eq!(seen[1].full, Some(PhenixValue::U64(8)));
}
