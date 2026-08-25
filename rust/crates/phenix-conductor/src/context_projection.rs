use crate::{ConductorError, ConductorRuntime, DomainEvent, ResolvedExactReference};
use phenix_core::{
    ConfigRevisionId, ContextDescriptor, ContextInjectionLifetime, ContextInjectionRequester,
    ContextRevision, ExactReference, ExecutionAuthority, ExecutionId, SkillId,
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
pub struct ExecutionContextProjection {
    pub execution_id: ExecutionId,
    pub config_revision: ConfigRevisionId,
    pub authority: ExecutionAuthority,
    pub catalog: Vec<ContextDescriptor>,
    pub injections: Vec<ContextProjectionInspection>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContextProjectionAccounting {
    pub catalog_estimated_cost: u64,
    pub injected_content_bytes: u64,
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
        let mut injections = Vec::new();
        for entry in &runtime.journal.entries {
            let DomainEvent::ContextInjectionRecorded { injection } = &entry.event else {
                continue;
            };
            if injection.execution_id != *execution_id {
                continue;
            }
            injections.push(ContextProjectionInspection {
                source_ref: injection.source_ref.clone(),
                source_revision: injection.source_revision.clone(),
                requested_by: injection.requested_by.clone(),
                reason: injection.reason.clone(),
                lifetime: injection.lifetime.clone(),
                content_identity: injection.content_identity.clone(),
                content: exact_context_content(runtime, &injection.source_ref)?,
            });
        }

        Ok(ExecutionContextProjection {
            execution_id: execution_id.clone(),
            config_revision,
            authority,
            catalog,
            injections,
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
        let projection = Self::project_execution(runtime, execution_id)?;
        let (prompt, _) = Self::render_model_prompt(runtime, execution_id, input)?;
        Ok(ContextProjectionAccounting {
            catalog_estimated_cost: projection.estimated_catalog_cost(),
            injected_content_bytes: projection.injected_content_bytes(),
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
    let injected = projection
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
    if injected.is_empty() {
        return base_prompt;
    }

    let injection_block = format!("<injected_context>\n{injected}</injected_context>\n");
    if let Some(index) = base_prompt.find("</phenix_context>") {
        let mut output = base_prompt;
        output.insert_str(index, &injection_block);
        return output;
    }

    format!(
        "<phenix_context>\n{injection_block}</phenix_context>\n\n<user_request>\n{}\n</user_request>",
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
        ExecutionTarget, InferenceOptions, ModelId, ModelTarget, ProviderId,
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
        assert!(accounting.catalog_estimated_cost >= accounting.injected_content_bytes);

        fs::remove_dir_all(root).unwrap();
    }
}
