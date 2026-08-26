#![forbid(unsafe_code)]

mod artifacts;
mod context;
mod planning;
mod repository_workers;
mod sessions;

pub use artifacts::{
    artifact_factory, artifact_manifest, artifact_service, ArtifactCommand, ArtifactProvenance,
    ArtifactRecord, ArtifactResponse, NormalizedReadRequest, ReadProviderIdentity,
    ReadResultRecord, RevalidationRecord, RevalidationVerdict, ARTIFACT_SERVICE,
};
pub use context::{
    context_factory, context_manifest, context_service, ContextCommand, ContextDescriptor,
    ContextInjection, ContextInjectionLifetime, ContextInjectionRequester, ContextResourceKind,
    ContextResourceRevision, ContextResponse, ContextScope, ExactContextReference,
    ExecutionContextProjection, ProjectedContextEntry, RepositoryContextSource, CONTEXT_SERVICE,
};
pub use planning::{
    planning_factory, planning_manifest, planning_service, DecisionRecord, HistoryEntry, HistoryKind,
    ObjectiveRecord, PlanRecord, PlanStep, PlanningCommand, PlanningResponse, PLANNING_SERVICE,
};
pub use repository_workers::{
    ReconstructedPullRequest, RepositoryCheckState, RepositoryChecklistEvidence,
    RepositoryDiscussionEvidence, RepositoryDiscussionKind, RepositoryFinding,
    RepositoryIssueCluster, RepositoryIssueEvidence, RepositoryPullRequestEvidence,
    RepositoryPullRequestState, RepositorySelectionReason, RepositoryValidation,
    RepositoryWorkPriority, RepositoryWorkSelection, RepositoryWorkSnapshot, RepositoryWorkerQueue,
};
pub use sessions::{
    session_factory, session_manifest, session_service, SessionCommand, SessionRecord,
    SessionResponse, SESSION_SERVICE,
};

use phenix_kernel::{
    Authority, PluginExecution, PluginHost, PluginId, PluginInstance, PluginManifest,
    ServiceContribution, ServiceId,
};
use serde_json::json;

pub const REPOSITORY_WORK_QUEUE_SERVICE: &str = "phenix.repository.worker-queue@1";
const REPOSITORY_WORKER_PLUGIN: &str = "phenix.repository-workers";

#[must_use]
pub fn repository_worker_manifest() -> PluginManifest {
    PluginManifest {
        id: PluginId::parse(REPOSITORY_WORKER_PLUGIN).expect("static plugin id is valid"),
        version: 1,
        execution: PluginExecution::Embedded,
        dependencies: Vec::new(),
        services: vec![ServiceContribution {
            service: repository_work_queue_service(),
            priority: 100,
            required_authority: Authority::default(),
        }],
        resource_namespaces: Vec::new(),
        maximum_authority: Authority::default(),
    }
}

#[must_use]
pub fn repository_worker_factory() -> Box<dyn PluginInstance> {
    Box::new(RepositoryWorkerPlugin)
}

#[must_use]
pub fn repository_work_queue_service() -> ServiceId {
    ServiceId::parse(REPOSITORY_WORK_QUEUE_SERVICE).expect("static service id is valid")
}

struct RepositoryWorkerPlugin;

impl PluginInstance for RepositoryWorkerPlugin {
    fn start(&mut self, _host: &PluginHost<'_>) -> Result<(), String> {
        Ok(())
    }

    fn invoke(
        &mut self,
        service: &ServiceId,
        input: &[u8],
        _host: &PluginHost<'_>,
    ) -> Result<Vec<u8>, String> {
        if service != &repository_work_queue_service() {
            return Err(format!("unsupported repository worker service: {service}"));
        }

        let snapshot: RepositoryWorkSnapshot =
            serde_json::from_slice(input).map_err(|error| error.to_string())?;
        let queue = RepositoryWorkerQueue::reconstruct(&snapshot);
        let result = match queue.select_work() {
            Some(selection) => json!({
                "pr_number": selection.pr_number,
                "reason": selection_reason_name(selection.reason),
            }),
            None => serde_json::Value::Null,
        };
        serde_json::to_vec(&result).map_err(|error| error.to_string())
    }
}

fn selection_reason_name(reason: RepositorySelectionReason) -> &'static str {
    match reason {
        RepositorySelectionReason::BrokenValidation => "broken_validation",
        RepositorySelectionReason::BlockingFinding => "blocking_finding",
        RepositorySelectionReason::StaleOrIncomplete => "stale_or_incomplete",
        RepositorySelectionReason::MissingContractEvidence => "missing_contract_evidence",
        RepositorySelectionReason::DependencyBlocking => "dependency_blocking",
        RepositorySelectionReason::NextReady => "next_ready",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use phenix_kernel::{KernelConfig, PluginState};
    use std::collections::BTreeSet;

    fn success_validation() -> RepositoryValidation {
        RepositoryValidation {
            source: RepositoryCheckState::Success,
            rust: RepositoryCheckState::Success,
            product: RepositoryCheckState::Success,
            integration_system: RepositoryCheckState::Success,
            maintenance: RepositoryCheckState::Success,
            maintenance_autofix: RepositoryCheckState::Success,
        }
    }

    fn snapshot() -> RepositoryWorkSnapshot {
        RepositoryWorkSnapshot {
            pull_requests: vec![RepositoryPullRequestEvidence {
                number: 42,
                semantic_key: "plugin-suite".into(),
                queue_order: 42,
                state: RepositoryPullRequestState::Open,
                draft: false,
                head_sha: "head".into(),
                base_sha: "main".into(),
                base_is_current: true,
                dependencies: BTreeSet::new(),
                contract_markdown: "- [ ] implementation".into(),
                checklist_evidence: vec![RepositoryChecklistEvidence {
                    item: "implementation".into(),
                    proven: true,
                }],
                discussions: Vec::new(),
                missing_regression: false,
                missing_spec_or_invariant: false,
                validation: success_validation(),
            }],
            issues: Vec::new(),
        }
    }

    #[test]
    fn repository_worker_behavior_runs_through_the_kernel_service_contract() {
        let manifest = repository_worker_manifest();
        let plugin = manifest.id.clone();
        let mut kernel = phenix_kernel::Kernel::new(KernelConfig::new([manifest]).unwrap());
        kernel
            .register_embedded_factory(plugin.clone(), repository_worker_factory)
            .unwrap();
        kernel.activate_all().unwrap();
        assert_eq!(kernel.state(&plugin), Some(PluginState::Active));

        let input = serde_json::to_vec(&snapshot()).unwrap();
        let output = kernel
            .invoke(
                &repository_work_queue_service(),
                &input,
                &Authority::default(),
                None,
            )
            .unwrap();
        let output: serde_json::Value = serde_json::from_slice(&output).unwrap();
        assert_eq!(output["pr_number"], 42);
        assert_eq!(output["reason"], "next_ready");
    }
}
