pub struct ObservableTransaction<'state, 'roots> {
    state: &'state mut StoreState,
    declared: SmallVec<[&'roots ValueId; 4]>,
    undo: SmallVec<[UndoRecord<'roots>; 8]>,
    touched: SmallVec<[TouchedMutation<'roots>; 8]>,
}

#[derive(Clone)]
#[allow(clippy::large_enum_variant)]
enum TouchedKind {
    Replace,
    Remove,
    Splice {
        start: u32,
        delete_count: u32,
        inserted: SmallVec<[PhenixValue; 4]>,
    },
}

#[derive(Clone)]
struct TouchedMutation<'roots> {
    value: &'roots ValueId,
    path: ValuePath,
    kind: TouchedKind,
}

#[allow(clippy::large_enum_variant)]
enum UndoRecord<'roots> {
    Replace {
        value: &'roots ValueId,
        path: ValuePath,
        previous: Option<PhenixValue>,
    },
    Splice {
        value: &'roots ValueId,
        path: ValuePath,
        start: usize,
        inserted_count: usize,
        removed: SmallVec<[PhenixValue; 4]>,
    },
}
