use super::*;
use phenix_core::{ObservableRef, ValueId};

variants!(ObservablePathSegment, "phenix.application.type.observable-path-segment@1", {
    Field { key: String },
    MapKey { key: String },
    Index { index: u64 },
    VariantPayload,
    OptionPayload,
});

record!(ObservablePath, "phenix.application.type.observable-path@1", {
    segments: Vec<ObservablePathSegment>,
});

record!(ObservableAddress, "phenix.application.type.observable-address@1", {
    value_id: ValueId,
    path: ObservablePath,
});

variants!(ObservableScope, "phenix.application.type.observable-scope@1", {
    Exact,
    Recursive,
});

variants!(ObservableMode, "phenix.application.type.observable-mode@1", {
    Diff,
    Full,
});

variants!(ObservableInitial, "phenix.application.type.observable-initial@1", {
    None,
    Full,
});

variants!(ObservableSnapshotPolicy, "phenix.application.type.observable-snapshot-policy@1", {
    CurrentOnly,
    CopyOnChange,
});

record!(ObservableResource, "phenix.application.type.observable-resource@1", {
    namespace: String,
    resource: String,
    binding_path: Vec<String>,
    reference: ObservableRef,
    value_id: ValueId,
    path: ObservablePath,
    schema: PhenixSchema,
    version: u64,
    snapshot_policy: ObservableSnapshotPolicy,
});

record!(ObservableList, "phenix.application.type.observable-list@1", {
    resources: Vec<ObservableResource>,
});

record!(ObservableGetInput, "phenix.application.type.observable-get-input@1", {
    reference: ObservableRef,
    path: ObservablePath,
});

record!(ObservableValue, "phenix.application.type.observable-value@1", {
    value_id: ValueId,
    version: u64,
    value: PhenixValue,
});

record!(ObservableSubscribeInput, "phenix.application.type.observable-subscribe-input@1", {
    reference: ObservableRef,
    path: ObservablePath,
    scope: ObservableScope,
    mode: ObservableMode,
    initial: ObservableInitial,
});

record!(ObservableUnsubscribeInput, "phenix.application.type.observable-unsubscribe-input@1", {
    subscription_id: u64,
    generation: u64,
});

record!(ObservableReplace, "phenix.application.type.observable-replace@1", {
    path: ObservablePath,
    value: PhenixValue,
});

record!(ObservableRemove, "phenix.application.type.observable-remove@1", {
    path: ObservablePath,
});

record!(ObservableSplice, "phenix.application.type.observable-splice@1", {
    path: ObservablePath,
    start: u64,
    delete_count: u64,
    inserted: Vec<PhenixValue>,
});

variants!(ObservableChange, "phenix.application.type.observable-change@1", {
    Replace { change: ObservableReplace },
    Remove { change: ObservableRemove },
    Splice { change: ObservableSplice },
});

variants!(ObservablePayload, "phenix.application.type.observable-payload@1", {
    Diff { changes: Vec<ObservableChange> },
    Full { value: PhenixValue },
});

record!(ObservableDelivery, "phenix.application.type.observable-delivery@1", {
    commit_id: Option<u64>,
    value_id: ValueId,
    from_version: u64,
    version: u64,
    address: ObservableAddress,
    subscription_id: u64,
    generation: u64,
    payload: ObservablePayload,
});

record!(ObservableSubscriptionResult, "phenix.application.type.observable-subscription-result@1", {
    subscription_id: u64,
    generation: u64,
    initial: Option<ObservableDelivery>,
});
