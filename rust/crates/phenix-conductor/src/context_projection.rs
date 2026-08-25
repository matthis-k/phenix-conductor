use crate::{
    ConductorError, ConductorRuntime, ContextCheckpoint, DomainEvent, ResolvedExactReference,
};
use phenix_core::{
    ConfigRevisionId, ContextDescriptor, ContextInjectionLifetime, ContextInjectionRequester,
    ContextResourceKind, ContextRevision, ContextScope, ExactReference, ExecutionAuthority,
    ExecutionId, SkillId,
};
use std::collections::BTreeSet;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContextProjectionInspection {
    pub source_ref: ExactReference,
    pub source_revision: ContextRevision,
    pub requested_by: ContextInjectionRequester,
    pub reason: String,
    pub lifetime: ContextInjectionLifetime,
    pub content_identity: ContextRevision,
    pub content: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContextArtifactView {
    pub recovery_ref: ExactReference,
    pub revision: ContextRevision,
    pub title: String,
    pub estimated_cost: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ContextPruneReason {
    ArtifactBodyCompacted,
    RepeatedExactInjection,
    CheckpointCompacted,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContextPruneInspection {
    pub reason: ContextPruneReason,
    pub recovery_ref: ExactReference,
    pub content_identity: ContextRevision,
    pub original_bytes: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExecutionContextProjection {
    pub execution_id: ExecutionId,
    pub config_revision: ConfigRevisionId,
    pub authority: ExecutionAuthority,
    pub catalog: Vec<ContextDescriptor>,
    pub injections: Vec<ContextProjectionInspection>,
    pub artifacts: Vec<ContextArtifactView>,
    pub pruned: Vec<ContextPruneInspection>,
    pub checkpoint: Option<ContextCheckpoint>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContextProjectionAccounting {
    pub catalog_estimated_cost: u64,
    pub base_prompt_bytes: u64,
    pub injected_content_bytes: u64,
    pub injected_context_bytes: u64,
    pub artifact_descriptor_bytes: u64,
    pub rendered_prompt_bytes: u64,
}

impl ExecutionContextProjection {
    #[must_use]
    pub fn estimated_catalog_cost(&self) -> u64 {
        self.catalog
            .iter()
            .map(|descriptor| descriptor.estimated_cost)
            .sum()
    }

    #[must_use]
    pub fn injected_content_bytes(&self) -> u64 {
        self.injections
            .iter()
            .filter_map(|injection| injection.content.as_ref())
            .map(|content| content.len() as u64)
            .sum()
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct ContextManager;

impl ContextManager {
    pub fn project_execution(
        runtime: &ConductorRuntime,
        execution_id: &ExecutionId,
    ) -> Result<ExecutionContextProjection, ConductorError> {
        let config_revision = runtime.execution_config_revision(execution_id)?;
        let authority = runtime.execution_authority(execution_id)?;
        let mut catalog = runtime.context_descriptors_for_execution(execution_id)?;
        catalog.sort_by(|left, right| {
            left.id
                .cmp(&right.id)
                .then_with(|| left.revision.cmp(&right.revision))
        });

        let checkpoint = runtime
            .latest_context_checkpoint(execution_id)
            .map(|(_, checkpoint)| checkpoint.clone());
        let mut injections = Vec::new();
        let mut artifacts = Vec::new();
        let mut pruned = Vec::new();
        for entry in &runtime.journal.entries {
            match &entry.event {
                DomainEvent::ContextInjectionRecorded { injection }
                    if injection.execution_id == *execution_id =>
                {
                    let mut content = exact_context_content(runtime, &injection.source_ref)?;
                    if checkpoint.as_ref().is_some_and(|checkpoint| {
                        checkpoint
                            .covered_history
                            .iter()
                            .any(|range| range.contains(entry.sequence))
                    }) {
                        let original_bytes = content.as_ref().map_or(0, |value| value.len() as u64);
                        pruned.push(ContextPruneInspection {
                            reason: ContextPruneReason::CheckpointCompacted,
                            recovery_ref: injection.source_ref.clone(),
                            content_identity: injection.content_identity.clone(),
                            original_bytes,
                        });
                        content = None;
                    }
                    injections.push(ContextProjectionInspection {
                        source_ref: injection.source_ref.clone(),
                        source_revision: injection.source_revision.clone(),
                        requested_by: injection.requested_by.clone(),
                        reason: injection.reason.clone(),
                        lifetime: injection.lifetime.clone(),
                        content_identity: injection.content_identity.clone(),
                        content,
                    });
                }
                DomainEvent::ContextResourceRevisionRegistered { resource }
                    if resource.descriptor.kind == ContextResourceKind::Artifact
                        && matches!(
                            &resource.descriptor.scope,
                            ContextScope::Execution { execution_id: owner } if owner == execution_id
                        ) =>
                {
                    artifacts.push(ContextArtifactView {
                        recovery_ref: resource.source_ref.clone(),
                        revision: resource.descriptor.revision.clone(),
                        title: resource.descriptor.title.clone(),
                        estimated_cost: resource.descriptor.estimated_cost,
                    });
                    pruned.push(ContextPruneInspection {
                        reason: ContextPruneReason::ArtifactBodyCompacted,
                        recovery_ref: resource.source_ref.clone(),
                        content_identity: resource.content_identity.clone(),
                        original_bytes: resource
                            .content
                            .as_ref()
                            .map_or(0, |content| content.len() as u64),
                    });
                }
                _ => {}
            }
        }
        artifacts.sort_by(|left, right| {
            left.recovery_ref
                .to_string()
                .cmp(&right.recovery_ref.to_string())
                .then_with(|| left.revision.as_str().cmp(right.revision.as_str()))
        });
        pruned.sort_by(|left, right| {
            left.recovery_ref
                .to_string()
                .cmp(&right.recovery_ref.to_string())
                .then_with(|| {
                    left.content_identity
                        .as_str()
                        .cmp(right.content_identity.as_str())
                })
        });

        for index in 0..injections.len() {
            let repeated = injections[index].content.is_some()
                && injections[index + 1..].iter().any(|later| {
                    later.source_ref == injections[index].source_ref
                        && later.content_identity == injections[index].content_identity
                });
            if repeated {
                let original_bytes = injections[index]
                    .content
                    .as_ref()
                    .map_or(0, |content| content.len() as u64);
                pruned.push(ContextPruneInspection {
                    reason: ContextPruneReason::RepeatedExactInjection,
                    recovery_ref: injections[index].source_ref.clone(),
                    content_identity: injections[index].content_identity.clone(),
                    original_bytes,
                });
                injections[index].content = None;
            }
        }

        Ok(ExecutionContextProjection {
            execution_id: execution_id.clone(),
            config_revision,
            authority,
            catalog,
            injections,
            artifacts,
            pruned,
            checkpoint,
        })
    }

    pub(crate) fn render_model_prompt(
        runtime: &ConductorRuntime,
        execution_id: &ExecutionId,
        input: &str,
    ) -> Result<(String, BTreeSet<SkillId>), ConductorError> {
        let configuration = runtime.configuration_for_execution(execution_id)?;
        let (base_prompt, explicit_skills) = configuration
            .context
            .compose_prompt_with_activations(&configuration.skills, input)?;
        let projection = Self::project_execution(runtime, execution_id)?;
        Ok((
            render_projection_prompt(base_prompt, &projection),
            explicit_skills,
        ))
    }

    pub fn account_execution(
        runtime: &ConductorRuntime,
        execution_id: &ExecutionId,
        input: &str,
    ) -> Result<ContextProjectionAccounting, ConductorError> {
        let configuration = runtime.configuration_for_execution(execution_id)?;
        let (base_prompt, _) = configuration
            .context
            .compose_prompt_with_activations(&configuration.skills, input)?;
        let projection = Self::project_execution(runtime, execution_id)?;
        let injected_context = render_injected_context(&projection);
        let artifact_descriptors = render_artifact_descriptors(&projection);
        let prompt = render_projection_prompt_sections(
            base_prompt.clone(),
            &injected_context,
            &artifact_descriptors,
        );
        Ok(ContextProjectionAccounting {
            catalog_estimated_cost: projection.estimated_catalog_cost(),
            base_prompt_bytes: base_prompt.len() as u64,
            injected_content_bytes: projection.injected_content_bytes(),
            injected_context_bytes: injected_context.len() as u64,
            artifact_descriptor_bytes: artifact_descriptors.len() as u64,
            rendered_prompt_bytes: prompt.len() as u64,
        })
    }
}

impl ConductorRuntime {
    pub fn project_execution_context(
        &self,
        execution_id: &ExecutionId,
    ) -> Result<ExecutionContextProjection, ConductorError> {
        ContextManager::project_execution(self, execution_id)
    }

    pub(crate) fn render_model_prompt(
        &self,
        execution_id: &ExecutionId,
        input: &str,
    ) -> Result<(String, BTreeSet<SkillId>), ConductorError> {
        ContextManager::render_model_prompt(self, execution_id, input)
    }

    pub fn account_execution_context(
        &self,
        execution_id: &ExecutionId,
        input: &str,
    ) -> Result<ContextProjectionAccounting, ConductorError> {
        ContextManager::account_execution(self, execution_id, input)
    }
}

fn exact_context_content(
    runtime: &ConductorRuntime,
    reference: &ExactReference,
) -> Result<Option<String>, ConductorError> {
    match runtime.resolve_exact_reference(reference)? {
        ResolvedExactReference::Context(resource) => Ok(resource.content),
        ResolvedExactReference::Objective(objective) => Ok(Some(
            serde_json::to_string(&objective).expect("objective context must serialize"),
        )),
        ResolvedExactReference::Plan(plan) => Ok(Some(
            serde_json::to_string(&plan).expect("plan context must serialize"),
        )),
        _ => Ok(None),
    }
}

fn render_projection_prompt(
    base_prompt: String,
    projection: &ExecutionContextProjection,
) -> String {
    let injected_context = render_injected_context(projection);
    let artifact_descriptors = render_artifact_descriptors(projection);
    render_projection_prompt_sections(base_prompt, &injected_context, &artifact_descriptors)
}

fn render_injected_context(projection: &ExecutionContextProjection) -> String {
    let checkpoint = projection
        .checkpoint
        .as_ref()
        .map_or_else(String::new, |checkpoint| {
            format!(
                "<checkpoint model=\"{}/{}/{}\">\n{}\n</checkpoint>\n",
                escape_xml(checkpoint.generation.model.backend.as_str()),
                escape_xml(checkpoint.generation.model.provider.as_str()),
                escape_xml(checkpoint.generation.model.model.as_str()),
                escape_xml(checkpoint.summary.trim())
            )
        });
    let resources = projection
        .injections
        .iter()
        .filter_map(|injection| {
            injection.content.as_ref().map(|content| {
                format!(
                    "<resource source=\"{}\" revision=\"{}\">\n{}\n</resource>\n",
                    escape_xml(&injection.source_ref.to_string()),
                    escape_xml(injection.source_revision.as_str()),
                    escape_xml(content.trim())
                )
            })
        })
        .collect::<String>();
    if resources.is_empty() && checkpoint.is_empty() {
        String::new()
    } else {
        format!("<injected_context>\n{checkpoint}{resources}</injected_context>\n")
    }
}

fn render_artifact_descriptors(projection: &ExecutionContextProjection) -> String {
    let artifacts = projection
        .artifacts
        .iter()
        .map(|artifact| {
            format!(
                "<artifact source=\"{}\" revision=\"{}\" title=\"{}\" />\n",
                escape_xml(&artifact.recovery_ref.to_string()),
                escape_xml(artifact.revision.as_str()),
                escape_xml(&artifact.title)
            )
        })
        .collect::<String>();
    if artifacts.is_empty() {
        String::new()
    } else {
        format!("<artifacts>\n{artifacts}</artifacts>\n")
    }
}

fn render_projection_prompt_sections(
    base_prompt: String,
    injected_context: &str,
    artifact_descriptors: &str,
) -> String {
    let projection_block = format!("{injected_context}{artifact_descriptors}");
    if projection_block.is_empty() {
        return base_prompt;
    }

    if let Some(index) = base_prompt.find("</phenix_context>") {
        let mut output = base_prompt;
        output.insert_str(index, &projection_block);
        return output;
    }

    format!(
        "<phenix_context>\n{projection_block}</phenix_context>\n\n<user_request>\n{}\n</user_request>",
        escape_xml(base_prompt.trim_start())
    )
}

fn escape_xml(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for character in value.chars() {
        match character {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&apos;"),
            _ => escaped.push(character),
        }
    }
    escaped
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CompiledConfiguration, ContextRegistry, SkillRegistry, SqliteStore};
    use phenix_core::{
        BackendId, ContextInjectionLifetime, ContextInjectionRequester, ContextResourceId,
        ExecutionEventKind, ExecutionTarget, InferenceOptions, ModelId, ModelTarget, ProviderId,
        ToolCallId,
    };
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    fn fixture_root() -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("phenix-context-projection-{nonce}"))
    }

    fn write(path: impl AsRef<Path>, content: &str) {
        let path = path.as_ref();
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, content).unwrap();
    }

    fn fixed_target() -> ExecutionTarget {
        ExecutionTarget::Fixed(ModelTarget {
            backend: BackendId::parse("mock").unwrap(),
            provider: ProviderId::parse("mock").unwrap(),
            model: ModelId::parse("mock").unwrap(),
            inference: InferenceOptions::default(),
        })
    }

    fn configuration_for(root: &Path) -> CompiledConfiguration {
        let mut configuration = CompiledConfiguration::default();
        configuration.install_context_registry(ContextRegistry::discover(root).unwrap());
        configuration.install_skill_registry(SkillRegistry::discover(root).unwrap());
        configuration
    }

    #[test]
    fn projection_is_deterministic_and_preserves_authority() {
        let root = fixture_root();
        fs::create_dir_all(root.join(".git")).unwrap();
        write(root.join("CONTRIBUTING.md"), "projection context");

        let mut runtime = ConductorRuntime::new();
        runtime
            .reload_configuration(configuration_for(&root))
            .unwrap();
        let session = runtime.create_session(None, None, fixed_target()).unwrap();
        let execution = runtime.submit(&session.id, "inspect projection").unwrap();
        let authority = runtime.execution_authority(&execution.id).unwrap();
        let resource_id = ContextResourceId::parse("project-document:CONTRIBUTING.md").unwrap();
        let descriptor = runtime
            .context_descriptors_for_execution(&execution.id)
            .unwrap()
            .into_iter()
            .find(|descriptor| descriptor.id == resource_id)
            .unwrap();
        runtime
            .load_context_for_execution(
                &execution.id,
                &resource_id,
                &descriptor.revision,
                ContextInjectionRequester::Agent,
                ContextInjectionLifetime::Execution,
                "projection regression",
            )
            .unwrap();

        let first = runtime.project_execution_context(&execution.id).unwrap();
        let second = runtime.project_execution_context(&execution.id).unwrap();
        assert_eq!(first, second);
        assert_eq!(first.authority, authority);
        assert_eq!(first.injections.len(), 1);
        assert_eq!(first.injections[0].reason, "projection regression");
        assert_eq!(
            first.injections[0].requested_by,
            ContextInjectionRequester::Agent
        );
        assert_eq!(first.injections[0].source_revision, descriptor.revision);
        assert_eq!(
            first.injections[0].content.as_deref(),
            Some("projection context")
        );
        assert_eq!(
            first.injected_content_bytes(),
            "projection context".len() as u64
        );
        assert!(first.catalog.iter().any(|item| item.id == resource_id));
        assert!(first.artifacts.is_empty());
        assert!(first.pruned.is_empty());

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn projection_restores_from_relational_state() {
        let root = fixture_root();
        fs::create_dir_all(root.join(".git")).unwrap();
        write(root.join("CONTRIBUTING.md"), "restored projection context");

        let mut runtime = ConductorRuntime::new();
        let revision = runtime
            .reload_configuration(configuration_for(&root))
            .unwrap();
        let session = runtime.create_session(None, None, fixed_target()).unwrap();
        let execution = runtime.submit(&session.id, "restore projection").unwrap();
        let resource_id = ContextResourceId::parse("project-document:CONTRIBUTING.md").unwrap();
        let descriptor = runtime
            .context_descriptors_for_execution(&execution.id)
            .unwrap()
            .into_iter()
            .find(|descriptor| descriptor.id == resource_id)
            .unwrap();
        runtime
            .load_context_for_execution(
                &execution.id,
                &resource_id,
                &descriptor.revision,
                ContextInjectionRequester::Agent,
                ContextInjectionLifetime::Execution,
                "restore projection regression",
            )
            .unwrap();
        let before = runtime.project_execution_context(&execution.id).unwrap();

        let store = SqliteStore::new(root.join("state.sqlite"));
        store.save(runtime.journal()).unwrap();
        let mut restored = ConductorRuntime::restore(store.load().unwrap()).unwrap();
        restored
            .bind_configuration_revision(&revision, configuration_for(&root))
            .unwrap();
        let after = restored.project_execution_context(&execution.id).unwrap();

        assert_eq!(after, before);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn projections_do_not_share_injections_between_executions() {
        let root = fixture_root();
        fs::create_dir_all(root.join(".git")).unwrap();
        write(root.join("CONTRIBUTING.md"), "execution scoped projection");

        let mut runtime = ConductorRuntime::new();
        runtime
            .reload_configuration(configuration_for(&root))
            .unwrap();
        let session = runtime.create_session(None, None, fixed_target()).unwrap();
        let left = runtime.submit(&session.id, "left").unwrap();
        let right = runtime.submit(&session.id, "right").unwrap();
        let resource_id = ContextResourceId::parse("project-document:CONTRIBUTING.md").unwrap();
        let descriptor = runtime
            .context_descriptors_for_execution(&left.id)
            .unwrap()
            .into_iter()
            .find(|descriptor| descriptor.id == resource_id)
            .unwrap();
        runtime
            .load_context_for_execution(
                &left.id,
                &resource_id,
                &descriptor.revision,
                ContextInjectionRequester::Agent,
                ContextInjectionLifetime::Execution,
                "left only",
            )
            .unwrap();

        let left_projection = runtime.project_execution_context(&left.id).unwrap();
        let right_projection = runtime.project_execution_context(&right.id).unwrap();
        assert_eq!(left_projection.injections.len(), 1);
        assert!(right_projection.injections.is_empty());

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn model_prompt_renderer_uses_exact_injected_content() {
        let root = fixture_root();
        fs::create_dir_all(root.join(".git")).unwrap();
        write(root.join("CONTRIBUTING.md"), "frozen injected content");

        let mut runtime = ConductorRuntime::new();
        runtime
            .reload_configuration(configuration_for(&root))
            .unwrap();
        let session = runtime.create_session(None, None, fixed_target()).unwrap();
        let execution = runtime.submit(&session.id, "render projection").unwrap();
        let resource_id = ContextResourceId::parse("project-document:CONTRIBUTING.md").unwrap();
        let descriptor = runtime
            .context_descriptors_for_execution(&execution.id)
            .unwrap()
            .into_iter()
            .find(|descriptor| descriptor.id == resource_id)
            .unwrap();
        runtime
            .load_context_for_execution(
                &execution.id,
                &resource_id,
                &descriptor.revision,
                ContextInjectionRequester::Agent,
                ContextInjectionLifetime::Execution,
                "render exact content",
            )
            .unwrap();
        write(root.join("CONTRIBUTING.md"), "mutated source content");

        let (prompt, active_skills) = runtime
            .render_model_prompt(&execution.id, "render projection")
            .unwrap();
        assert!(active_skills.is_empty());
        assert!(prompt.contains("frozen injected content"));
        assert!(!prompt.contains("mutated source content"));
        assert!(prompt.contains(&descriptor.revision.to_string()));

        let resolved = runtime.resolve_invocation(&execution.id).unwrap();
        assert_eq!(resolved.prompt, prompt);
        let prepared = runtime
            .prepare_invocation(
                resolved,
                &phenix_backend::BackendCapabilities {
                    tool_presentations: BTreeSet::new(),
                    images: false,
                    persistent_sessions: false,
                },
            )
            .unwrap();
        assert_eq!(prepared.backend_execution_request().prompt, prompt);

        let accounting = runtime
            .account_execution_context(&execution.id, "render projection")
            .unwrap();
        assert_eq!(
            accounting.injected_content_bytes,
            "frozen injected content".len() as u64
        );
        assert_eq!(accounting.rendered_prompt_bytes, prompt.len() as u64);
        assert!(accounting.base_prompt_bytes > 0);
        assert!(accounting.injected_context_bytes > accounting.injected_content_bytes);
        assert_eq!(accounting.artifact_descriptor_bytes, 0);
        assert!(accounting.catalog_estimated_cost >= accounting.injected_content_bytes);

        let resolved = runtime.resolve_invocation(&execution.id).unwrap();
        assert_eq!(resolved.context_accounting, accounting);

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn promoted_build_artifact_is_pruned_to_a_recoverable_compact_view() {
        let mut runtime = ConductorRuntime::new();
        let session = runtime.create_session(None, None, fixed_target()).unwrap();
        let execution = runtime.submit(&session.id, "artifact projection").unwrap();
        let artifact = runtime
            .promote_text_artifact(&execution.id, "build log", "large exact build output")
            .unwrap();
        let journal_len = runtime.journal.entries.len();

        let projection = runtime.project_execution_context(&execution.id).unwrap();
        assert_eq!(projection.artifacts.len(), 1);
        assert_eq!(projection.artifacts[0].recovery_ref, artifact.source_ref);
        assert_eq!(
            projection.artifacts[0].revision,
            artifact.descriptor.revision
        );
        assert_eq!(projection.pruned.len(), 1);
        assert_eq!(
            projection.pruned[0].reason,
            ContextPruneReason::ArtifactBodyCompacted
        );
        assert_eq!(projection.pruned[0].recovery_ref, artifact.source_ref);
        assert_eq!(
            projection.pruned[0].original_bytes,
            "large exact build output".len() as u64
        );
        assert_eq!(runtime.journal.entries.len(), journal_len);

        let resolved = runtime
            .resolve_exact_reference(&projection.pruned[0].recovery_ref)
            .unwrap();
        assert_eq!(resolved, ResolvedExactReference::Context(artifact));

        let (prompt, _) = runtime
            .render_model_prompt(&execution.id, "artifact projection")
            .unwrap();
        assert!(prompt.contains("<artifacts>"));
        assert!(prompt.contains("build log"));
        assert!(!prompt.contains("large exact build output"));
    }

    #[test]
    fn small_tool_output_is_not_promoted_implicitly() {
        let mut runtime = ConductorRuntime::new();
        let session = runtime.create_session(None, None, fixed_target()).unwrap();
        let execution = runtime.submit(&session.id, "small output").unwrap();
        runtime
            .push_event(
                &execution.id,
                ExecutionEventKind::ToolCallFinished {
                    tool_call_id: ToolCallId::parse("tool-call-small").unwrap(),
                    output: "ok".to_owned(),
                    success: true,
                },
            )
            .unwrap();

        let projection = runtime.project_execution_context(&execution.id).unwrap();
        assert!(projection.artifacts.is_empty());
        assert!(projection.pruned.is_empty());
    }

    #[test]
    fn repeated_exact_injection_prunes_old_bytes_without_mutating_durable_state() {
        let root = fixture_root();
        fs::create_dir_all(root.join(".git")).unwrap();
        write(root.join("CONTRIBUTING.md"), "repeatable exact content");

        let mut runtime = ConductorRuntime::new();
        runtime
            .reload_configuration(configuration_for(&root))
            .unwrap();
        let session = runtime.create_session(None, None, fixed_target()).unwrap();
        let execution = runtime.submit(&session.id, "repeat context").unwrap();
        let resource_id = ContextResourceId::parse("project-document:CONTRIBUTING.md").unwrap();
        let descriptor = runtime
            .context_descriptors_for_execution(&execution.id)
            .unwrap()
            .into_iter()
            .find(|descriptor| descriptor.id == resource_id)
            .unwrap();
        for reason in ["first read", "second read"] {
            runtime
                .load_context_for_execution(
                    &execution.id,
                    &resource_id,
                    &descriptor.revision,
                    ContextInjectionRequester::Agent,
                    ContextInjectionLifetime::Execution,
                    reason,
                )
                .unwrap();
        }
        let journal_len = runtime.journal.entries.len();

        let projection = runtime.project_execution_context(&execution.id).unwrap();
        assert_eq!(projection.injections.len(), 2);
        assert!(projection.injections[0].content.is_none());
        assert_eq!(
            projection.injections[1].content.as_deref(),
            Some("repeatable exact content")
        );
        assert_eq!(projection.pruned.len(), 1);
        assert_eq!(
            projection.pruned[0].reason,
            ContextPruneReason::RepeatedExactInjection
        );
        assert_eq!(runtime.journal.entries.len(), journal_len);

        let recovered = runtime
            .resolve_exact_reference(&projection.pruned[0].recovery_ref)
            .unwrap();
        assert_eq!(
            recovered.context_resource().unwrap().content.as_deref(),
            Some("repeatable exact content")
        );

        fs::remove_dir_all(root).unwrap();
    }
}
