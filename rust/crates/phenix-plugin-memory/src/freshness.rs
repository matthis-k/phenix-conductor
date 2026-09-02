use phenix_core::ServiceId;
use phenix_sdk::{
    MemoryCanonicalReference, MemoryDependencyRevision, MemoryFreshness, MemoryFreshnessRecord,
    MemoryRecord, MemoryRevalidationOutcome,
};

pub(crate) fn initial_state(
    record: &MemoryRecord,
    canonical_reference: Option<MemoryCanonicalReference>,
) -> MemoryFreshnessRecord {
    let mut dependencies = record
        .source_refs
        .iter()
        .map(|source| MemoryDependencyRevision {
            service: source.service.clone(),
            resource: source.resource.clone(),
            revision: None,
        })
        .collect::<Vec<_>>();
    dependencies.sort();
    dependencies.dedup();

    MemoryFreshnessRecord {
        memory_id: record.id.clone(),
        freshness: MemoryFreshness::Current,
        changed_at: record.created_at,
        dependencies,
        canonical_reference,
    }
}

pub(crate) fn deterministic_outcome(
    record: &MemoryRecord,
    state: &MemoryFreshnessRecord,
    at: u64,
) -> MemoryRevalidationOutcome {
    if state.freshness == MemoryFreshness::Historical {
        return MemoryRevalidationOutcome::RetainHistorical;
    }
    if record.valid_until.is_some_and(|end| at >= end) {
        return MemoryRevalidationOutcome::Expire;
    }
    if state.freshness == MemoryFreshness::NeedsValidation {
        return MemoryRevalidationOutcome::NeedsValidation;
    }
    MemoryRevalidationOutcome::KeepCurrent
}

pub(crate) fn observe_revision_change(
    state: &mut MemoryFreshnessRecord,
    service: &ServiceId,
    resource: &str,
    revision: &str,
    at: u64,
) -> bool {
    let mut affected = false;
    for dependency in &mut state.dependencies {
        if dependency.service == *service && dependency.resource == resource {
            if dependency.revision.as_deref() != Some(revision) {
                affected = true;
            }
            dependency.revision = Some(revision.to_owned());
        }
    }
    if let Some(reference) = state.canonical_reference.as_mut() {
        if reference.service == *service && reference.resource == resource {
            if reference.revision.as_deref() != Some(revision) {
                affected = true;
            }
            reference.revision = Some(revision.to_owned());
        }
    }

    if affected && state.freshness == MemoryFreshness::Current {
        state.freshness = MemoryFreshness::NeedsValidation;
        state.changed_at = at;
    }
    affected
}

#[cfg(test)]
mod tests {
    use super::*;
    use phenix_core::{ServiceId, SessionId};
    use phenix_sdk::{MemoryKind, MemoryScope, MemorySourceReference};

    fn record(valid_until: Option<u64>) -> MemoryRecord {
        MemoryRecord {
            id: "fact".into(),
            kind: MemoryKind::Fact,
            scope: MemoryScope::Session {
                session_id: SessionId::parse("session-1").unwrap(),
            },
            content: "current fact".into(),
            source_refs: vec![MemorySourceReference {
                service: ServiceId::parse("fixture.history@1").unwrap(),
                resource: "turn/1".into(),
                start: None,
                end: None,
            }],
            supersedes: Vec::new(),
            valid_from: None,
            valid_until,
            created_at: 10,
        }
    }

    #[test]
    fn temporal_expiry_is_resolved_without_a_model_call() {
        let record = record(Some(20));
        let state = initial_state(&record, None);

        assert_eq!(
            deterministic_outcome(&record, &state, 20),
            MemoryRevalidationOutcome::Expire
        );
    }

    #[test]
    fn source_revision_change_marks_only_dependent_memory_for_validation() {
        let record = record(None);
        let mut state = initial_state(&record, None);
        state.dependencies[0].revision = Some("rev-1".into());
        let source_service = ServiceId::parse("fixture.history@1").unwrap();

        assert!(!observe_revision_change(
            &mut state,
            &source_service,
            "turn/other",
            "rev-2",
            20,
        ));
        assert_eq!(state.freshness, MemoryFreshness::Current);

        assert!(observe_revision_change(
            &mut state,
            &source_service,
            "turn/1",
            "rev-2",
            21,
        ));
        assert_eq!(state.freshness, MemoryFreshness::NeedsValidation);
        assert_eq!(state.changed_at, 21);
        assert_eq!(state.dependencies[0].revision.as_deref(), Some("rev-2"));
        assert!(!observe_revision_change(
            &mut state,
            &source_service,
            "turn/1",
            "rev-2",
            22,
        ));
    }

    #[test]
    fn canonical_reference_revision_participates_in_freshness() {
        let record = record(None);
        let planning = ServiceId::parse("phenix.planning@1").unwrap();
        let mut state = initial_state(
            &record,
            Some(MemoryCanonicalReference {
                service: planning.clone(),
                resource: "decision/7".into(),
                revision: Some("rev-1".into()),
            }),
        );

        assert!(observe_revision_change(
            &mut state,
            &planning,
            "decision/7",
            "rev-2",
            30,
        ));
        assert_eq!(state.freshness, MemoryFreshness::NeedsValidation);
        assert_eq!(
            state
                .canonical_reference
                .as_ref()
                .and_then(|reference| reference.revision.as_deref()),
            Some("rev-2")
        );
    }
}
