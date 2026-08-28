use crate::{default_suite_authority, HarnessBuildError, HarnessBuilder, PhenixHarness};
use phenix_core::{KernelError, PersistenceBackend};
use phenix_plugin_catalog::{
    basic_context_component_manifest, basic_context_factory, basic_context_manifest,
    basic_model_component_manifest, basic_model_factory, basic_model_manifest,
    basic_skills_component_manifest, basic_skills_factory, basic_skills_manifest,
    basic_tools_component_manifest, basic_tools_factory, basic_tools_manifest,
    session_component_manifest, session_factory, session_manifest,
};
use std::collections::BTreeSet;

impl HarnessBuilder {
    pub fn with_basic_suite() -> Result<Self, KernelError> {
        let mut builder = Self::new();
        builder.set_component_authority(default_suite_authority());
        builder.add_embedded(session_manifest(), session_factory)?;
        builder.add_embedded(basic_model_manifest(), basic_model_factory)?;
        builder.add_embedded(basic_tools_manifest(), basic_tools_factory)?;
        builder.add_embedded(basic_skills_manifest(), basic_skills_factory)?;
        builder.add_embedded(basic_context_manifest(), basic_context_factory)?;
        for component in [
            session_component_manifest(),
            basic_model_component_manifest(),
            basic_tools_component_manifest(),
            basic_skills_component_manifest(),
            basic_context_component_manifest(),
        ] {
            builder.add_component(component);
        }
        Ok(builder)
    }

    pub fn basic_suite_plugin_ids() -> BTreeSet<String> {
        [
            session_manifest(),
            basic_model_manifest(),
            basic_tools_manifest(),
            basic_skills_manifest(),
            basic_context_manifest(),
        ]
        .into_iter()
        .map(|manifest| manifest.id.as_str().to_owned())
        .collect()
    }
}

impl PhenixHarness {
    pub fn basic_suite() -> Result<Self, HarnessBuildError> {
        HarnessBuilder::with_basic_suite()?.build()
    }

    pub fn basic_suite_with_persistence(
        persistence: impl PersistenceBackend + 'static,
    ) -> Result<Self, HarnessBuildError> {
        HarnessBuilder::with_basic_suite()?.build_with_persistence(persistence)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use phenix_core::{
        context_service, model_inference_service, skill_service, tool_service, ContextCommand,
        ContextResourceKind, ContextResponse, ContextScope, LocalPersistence,
        ModelInferenceRequest, ModelInferenceResponse, SessionCommand, SessionResponse,
        SkillCommand, SkillDefinition, SkillResponse, ToolCommand, ToolDefinition, ToolResponse,
    };
    use phenix_plugin_catalog::session_service;
    use std::{
        collections::BTreeMap,
        fs,
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };

    fn temp_db() -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "phenix-basic-suite-{}-{nonce}.sqlite",
            std::process::id()
        ))
    }

    fn invoke<T: serde::Serialize, R: serde::de::DeserializeOwned>(
        harness: &mut PhenixHarness,
        service: &phenix_core::ServiceId,
        request: &T,
    ) -> R {
        let output = harness
            .invoke(
                service,
                &serde_json::to_vec(request).unwrap(),
                &default_suite_authority(),
                None,
            )
            .unwrap();
        serde_json::from_slice(&output).unwrap()
    }

    #[test]
    fn basic_suite_contains_only_independently_selected_basic_agent_plugins() {
        assert_eq!(
            HarnessBuilder::basic_suite_plugin_ids(),
            BTreeSet::from([
                "phenix.sessions".into(),
                "phenix.basic-model".into(),
                "phenix.basic-tools".into(),
                "phenix.basic-skills".into(),
                "phenix.basic-context".into(),
            ])
        );
        assert!(!HarnessBuilder::basic_suite_plugin_ids().contains("phenix.models"));
        assert!(!HarnessBuilder::basic_suite_plugin_ids().contains("phenix.context"));
    }

    #[test]
    fn selected_suite_accepts_each_basic_plugin_id() {
        for plugin in [
            "phenix.basic-model",
            "phenix.basic-tools",
            "phenix.basic-skills",
            "phenix.basic-context",
        ] {
            let selected = BTreeSet::from([plugin.to_owned()]);
            let harness = HarnessBuilder::with_selected_suite(&selected)
                .unwrap()
                .build()
                .unwrap();
            assert_eq!(
                harness
                    .kernel()
                    .config()
                    .manifests()
                    .map(|manifest| manifest.id.as_str())
                    .collect::<Vec<_>>(),
                vec![plugin]
            );
        }
    }

    #[test]
    fn minimal_agent_journey_uses_public_services_and_restores_plugin_owned_state() {
        let path = temp_db();
        {
            let persistence = LocalPersistence::open(&path).unwrap();
            let mut harness = PhenixHarness::basic_suite_with_persistence(persistence).unwrap();
            harness.activate().unwrap();

            let _: SessionResponse = invoke(
                &mut harness,
                &session_service(),
                &SessionCommand::Create { id: "root".into() },
            );
            let _: SkillResponse = invoke(
                &mut harness,
                &skill_service(),
                &SkillCommand::Register {
                    skill: SkillDefinition {
                        id: "review".into(),
                        content: b"review carefully".to_vec(),
                    },
                },
            );
            let _: ToolResponse = invoke(
                &mut harness,
                &tool_service(),
                &ToolCommand::Register {
                    tool: ToolDefinition {
                        id: "echo".into(),
                        input_schema: serde_json::json!({}),
                        output_schema: serde_json::json!({}),
                        output_prefix: b"tool:".to_vec(),
                    },
                },
            );
            let _: ContextResponse = invoke(
                &mut harness,
                &context_service(),
                &ContextCommand::Register {
                    resource_id: "project".into(),
                    kind: ContextResourceKind::ProjectDocument,
                    source: "README.md".into(),
                    scope: ContextScope::Workspace,
                    content: b"project context".to_vec(),
                },
            );
            let model: ModelInferenceResponse = invoke(
                &mut harness,
                &model_inference_service(),
                &ModelInferenceRequest {
                    model: "direct".into(),
                    input: b"hello".to_vec(),
                    options: BTreeMap::new(),
                },
            );
            assert_eq!(model.output, b"hello");
        }

        let persistence = LocalPersistence::open(&path).unwrap();
        let mut restored = PhenixHarness::basic_suite_with_persistence(persistence).unwrap();
        restored.activate().unwrap();
        let sessions: SessionResponse =
            invoke(&mut restored, &session_service(), &SessionCommand::List);
        assert!(
            matches!(sessions, SessionResponse::Sessions { sessions } if sessions[0].id == "root")
        );
        let skills: SkillResponse = invoke(&mut restored, &skill_service(), &SkillCommand::List);
        assert!(matches!(skills, SkillResponse::Skills { skills } if skills[0].id == "review"));
        let tools: ToolResponse = invoke(&mut restored, &tool_service(), &ToolCommand::List);
        assert!(matches!(tools, ToolResponse::Tools { tools } if tools[0].id == "echo"));
        let context: ContextResponse =
            invoke(&mut restored, &context_service(), &ContextCommand::List);
        assert!(
            matches!(context, ContextResponse::Resources { descriptors } if descriptors[0].resource_id == "project")
        );
        let _ = fs::remove_file(path);
    }
}
