use phenix_core::{
    Authority, CompatibilityMetadata, CompositionMetadataInput, GraphReconciler,
    MetadataChangeKind, ReconciliationAction, ReloadPolicy, ResourceMetadataChange,
    SkillResourceMetadata,
};
use std::collections::BTreeSet;

fn resource() -> SkillResourceMetadata {
    SkillResourceMetadata {
        identity: "fixture.skill".into(),
        version: 1,
        content_identity: "sha256:fixture".into(),
        dependencies: BTreeSet::new(),
        conflicts: BTreeSet::new(),
        triggers: BTreeSet::from(["review".into()]),
        scope: "execution".into(),
        priority: 0,
        required_tools: BTreeSet::new(),
        required_interfaces: BTreeSet::new(),
        required_capabilities: BTreeSet::new(),
        compatibility: CompatibilityMetadata {
            minimum_kernel_version: 1,
            maximum_kernel_version: None,
        },
        invalidation_targets: BTreeSet::from(["skill-index".into()]),
        reload_policy: ReloadPolicy::Restart,
    }
}

#[test]
fn metadata_only_resource_change_is_reconfigured_and_invalidates_declared_state() {
    let active_input = CompositionMetadataInput {
        packages: Vec::new(),
        components: Vec::new(),
        resources: vec![resource()],
        configuration: Vec::new(),
    };
    let mut candidate_input = active_input.clone();
    candidate_input.resources[0].priority = 10;

    let (active, active_metadata) = active_input
        .resolve_inspectable(&Authority::default())
        .unwrap();
    let (candidate, candidate_metadata) = candidate_input
        .resolve_inspectable(&Authority::default())
        .unwrap();
    let reconciler = GraphReconciler::new(active);

    let preview = reconciler
        .preview_candidate_with_metadata(&active_metadata, &candidate, &candidate_metadata)
        .unwrap();

    assert_eq!(
        preview.metadata.resources,
        vec![ResourceMetadataChange {
            resource: "fixture.skill".into(),
            kind: MetadataChangeKind::Reconfigured,
        }]
    );
    assert!(preview.transition_plan.contains(
        &ReconciliationAction::InvalidateResourceDerivedState {
            resource: "fixture.skill".into(),
            targets: BTreeSet::from(["skill-index".into()]),
        }
    ));
}
