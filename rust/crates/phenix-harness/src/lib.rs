use phenix_kernel::{
    Authority, CapabilityId, Kernel, KernelConfig, KernelError, PersistenceBackend,
    PluginExecution, PluginId, PluginInstance, PluginManifest,
};
use phenix_plugin_suite::{
    artifact_factory, artifact_manifest, cli_factory, cli_manifest, context_factory,
    context_manifest, debug_factory, debug_manifest, execution_factory, execution_manifest,
    frontend_factory, frontend_manifest, hook_factory, hook_manifest, job_factory, job_manifest,
    language_factory, language_manifest, model_routing_factory, model_routing_manifest,
    planning_factory, planning_manifest, repository_worker_factory, repository_worker_manifest,
    session_factory, session_manifest, workspace_factory, workspace_manifest,
};
use std::{collections::BTreeMap, sync::Arc};

type EmbeddedFactory = Arc<dyn Fn() -> Box<dyn PluginInstance> + Send + Sync>;
type ExternalFactory =
    Arc<dyn Fn(&PluginManifest) -> Result<Box<dyn PluginInstance>, String> + Send + Sync>;

pub fn default_suite_authority() -> Authority {
    Authority::new([
        CapabilityId::parse("kernel.persistence.schema").expect("static capability"),
        CapabilityId::parse("kernel.persistence.read").expect("static capability"),
        CapabilityId::parse("kernel.persistence.write").expect("static capability"),
        CapabilityId::parse("workspace.read").expect("static capability"),
        CapabilityId::parse("workspace.write").expect("static capability"),
        CapabilityId::parse("workspace.shell").expect("static capability"),
        CapabilityId::parse("workspace.git").expect("static capability"),
    ])
}

#[derive(Default)]
pub struct HarnessBuilder {
    manifests: Vec<PluginManifest>,
    embedded_factories: BTreeMap<PluginId, EmbeddedFactory>,
    external_factories: BTreeMap<PluginId, ExternalFactory>,
}

impl HarnessBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_default_suite() -> Result<Self, KernelError> {
        let mut builder = Self::new();
        let authority = default_suite_authority();
        builder.add_embedded(repository_worker_manifest(), repository_worker_factory)?;
        builder.add_embedded(session_manifest(), session_factory)?;
        builder.add_embedded(artifact_manifest(), artifact_factory)?;
        builder.add_embedded(cli_manifest(authority.clone()), cli_factory)?;
        builder.add_embedded(context_manifest(), context_factory)?;
        builder.add_embedded(execution_manifest(authority.clone()), execution_factory)?;
        builder.add_embedded(language_manifest(), language_factory)?;
        builder.add_embedded(planning_manifest(), planning_factory)?;
        builder.add_embedded(workspace_manifest(), workspace_factory)?;
        builder.add_embedded(
            model_routing_manifest(authority.clone()),
            model_routing_factory,
        )?;
        builder.add_embedded(job_manifest(), job_factory)?;
        builder.add_embedded(frontend_manifest(authority.clone()), frontend_factory)?;
        builder.add_embedded(hook_manifest(authority.clone()), hook_factory)?;
        builder.add_embedded(debug_manifest(authority), debug_factory)?;
        Ok(builder)
    }

    pub fn add_manifest(&mut self, manifest: PluginManifest) {
        self.manifests.push(manifest);
    }

    pub fn add_embedded<F>(
        &mut self,
        manifest: PluginManifest,
        factory: F,
    ) -> Result<(), KernelError>
    where
        F: Fn() -> Box<dyn PluginInstance> + Send + Sync + 'static,
    {
        if !matches!(manifest.execution, PluginExecution::Embedded) {
            return Err(KernelError::WrongExecutionKind(manifest.id));
        }
        let id = manifest.id.clone();
        self.manifests.push(manifest);
        self.embedded_factories.insert(id, Arc::new(factory));
        Ok(())
    }

    pub fn add_external<F>(
        &mut self,
        manifest: PluginManifest,
        factory: F,
    ) -> Result<(), KernelError>
    where
        F: Fn(&PluginManifest) -> Result<Box<dyn PluginInstance>, String> + Send + Sync + 'static,
    {
        if !matches!(manifest.execution, PluginExecution::External { .. }) {
            return Err(KernelError::WrongExecutionKind(manifest.id));
        }
        let id = manifest.id.clone();
        self.manifests.push(manifest);
        self.external_factories.insert(id, Arc::new(factory));
        Ok(())
    }

    pub fn build(self) -> Result<PhenixHarness, KernelError> {
        let config = KernelConfig::new(self.manifests)?;
        let mut kernel = Kernel::new(config);
        for (plugin, factory) in self.embedded_factories {
            kernel.register_embedded_factory(plugin, move || factory())?;
        }
        for (plugin, factory) in self.external_factories {
            kernel.register_external_factory(plugin, move |manifest| factory(manifest))?;
        }
        Ok(PhenixHarness { kernel })
    }

    pub fn build_with_persistence(
        self,
        persistence: impl PersistenceBackend + 'static,
    ) -> Result<PhenixHarness, KernelError> {
        let config = KernelConfig::new(self.manifests)?;
        let mut kernel = Kernel::with_persistence(config, persistence);
        for (plugin, factory) in self.embedded_factories {
            kernel.register_embedded_factory(plugin, move || factory())?;
        }
        for (plugin, factory) in self.external_factories {
            kernel.register_external_factory(plugin, move |manifest| factory(manifest))?;
        }
        Ok(PhenixHarness { kernel })
    }
}

pub struct PhenixHarness {
    kernel: Kernel,
}

impl PhenixHarness {
    pub fn kernel(&self) -> &Kernel {
        &self.kernel
    }

    pub fn kernel_mut(&mut self) -> &mut Kernel {
        &mut self.kernel
    }

    pub fn activate(&mut self) -> Result<(), KernelError> {
        self.kernel.activate_all()
    }

    pub fn kernel_only() -> Self {
        Self {
            kernel: Kernel::kernel_only(),
        }
    }

    pub fn default_suite() -> Result<Self, KernelError> {
        HarnessBuilder::with_default_suite()?.build()
    }

    pub fn default_suite_with_persistence(
        persistence: impl PersistenceBackend + 'static,
    ) -> Result<Self, KernelError> {
        HarnessBuilder::with_default_suite()?.build_with_persistence(persistence)
    }

    pub fn invoke(
        &mut self,
        service: &phenix_kernel::ServiceId,
        input: &[u8],
        authority: &Authority,
        binding: Option<&PluginId>,
    ) -> Result<Vec<u8>, KernelError> {
        self.kernel.invoke(service, input, authority, binding)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use phenix_kernel::{CapabilityId, ServiceContribution, ServiceId};
    use phenix_plugin_suite::{
        artifact_manifest, artifact_service, context_manifest, context_service, planning_manifest,
        planning_service, repository_work_queue_service, session_manifest, session_service,
        ArtifactCommand, ArtifactProvenance, ArtifactResponse, ContextCommand, ContextDescriptor,
        ContextResourceKind, ContextResponse, ContextScope, PlanningCommand, PlanningResponse,
        RepositoryWorkSnapshot, SessionCommand, SessionResponse,
    };

    fn plugin(value: &str) -> PluginId {
        PluginId::parse(value).unwrap()
    }

    fn capability(value: &str) -> CapabilityId {
        CapabilityId::parse(value).unwrap()
    }

    fn service() -> ServiceId {
        ServiceId::parse("fixture.echo@1").unwrap()
    }

    fn session_authority() -> Authority {
        session_manifest().maximum_authority
    }

    fn artifact_authority() -> Authority {
        artifact_manifest().maximum_authority
    }

    fn context_authority() -> Authority {
        context_manifest().maximum_authority
    }

    fn planning_authority() -> Authority {
        planning_manifest().maximum_authority
    }

    fn manifest(id: &str, priority: i32) -> PluginManifest {
        service_manifest(id, service(), priority, Authority::default())
    }

    fn service_manifest(
        id: &str,
        service: ServiceId,
        priority: i32,
        maximum_authority: Authority,
    ) -> PluginManifest {
        PluginManifest {
            id: plugin(id),
            version: 1,
            execution: PluginExecution::Embedded,
            dependencies: Vec::new(),
            services: vec![ServiceContribution {
                service,
                priority,
                required_authority: Authority::default(),
            }],
            resource_namespaces: Vec::new(),
            maximum_authority,
        }
    }

    struct Echo(&'static [u8]);

    impl PluginInstance for Echo {
        fn start(&mut self, _host: &phenix_kernel::PluginHost<'_>) -> Result<(), String> {
            Ok(())
        }

        fn invoke(
            &mut self,
            _service: &ServiceId,
            _input: &[u8],
            _host: &phenix_kernel::PluginHost<'_>,
        ) -> Result<Vec<u8>, String> {
            Ok(self.0.to_vec())
        }
    }

    struct FixedResponse(Vec<u8>);

    impl PluginInstance for FixedResponse {
        fn start(&mut self, _host: &phenix_kernel::PluginHost<'_>) -> Result<(), String> {
            Ok(())
        }

        fn invoke(
            &mut self,
            _service: &ServiceId,
            _input: &[u8],
            _host: &phenix_kernel::PluginHost<'_>,
        ) -> Result<Vec<u8>, String> {
            Ok(self.0.clone())
        }
    }

    #[test]
    fn kernel_only_harness_has_no_userspace_plugins() {
        let mut harness = PhenixHarness::kernel_only();
        harness.activate().unwrap();
        assert_eq!(harness.kernel().config().manifests().count(), 0);
        let input = serde_json::to_vec(&SessionCommand::Get {
            id: "missing".into(),
        })
        .unwrap();
        assert!(harness
            .invoke(&session_service(), &input, &session_authority(), None)
            .is_err());
        let context = serde_json::to_vec(&ContextCommand::List).unwrap();
        assert!(harness
            .invoke(&context_service(), &context, &context_authority(), None)
            .is_err());
        let planning = serde_json::to_vec(&PlanningCommand::GetObjective {
            id: "missing".into(),
        })
        .unwrap();
        assert!(harness
            .invoke(&planning_service(), &planning, &planning_authority(), None)
            .is_err());
    }

    #[test]
    fn default_harness_loads_first_party_suite_through_kernel_contracts() {
        let mut harness = PhenixHarness::default_suite().unwrap();
        harness.activate().unwrap();
        assert_eq!(harness.kernel().config().manifests().count(), 14);

        let input = serde_json::to_vec(&RepositoryWorkSnapshot {
            pull_requests: Vec::new(),
            issues: Vec::new(),
        })
        .unwrap();
        assert_eq!(
            harness
                .invoke(
                    &repository_work_queue_service(),
                    &input,
                    &Authority::default(),
                    None,
                )
                .unwrap(),
            b"null"
        );

        let create = serde_json::to_vec(&SessionCommand::Create {
            id: "session-1".into(),
            parent: None,
        })
        .unwrap();
        let response = harness
            .invoke(&session_service(), &create, &session_authority(), None)
            .unwrap();
        assert!(matches!(
            serde_json::from_slice::<SessionResponse>(&response).unwrap(),
            SessionResponse::Created { .. }
        ));

        let store = serde_json::to_vec(&ArtifactCommand::Store {
            content: b"readme".to_vec(),
            provenance: ArtifactProvenance {
                producer: "harness-smoke".into(),
                provider_identity: None,
                configuration_identity: None,
                source_observations: BTreeMap::new(),
            },
        })
        .unwrap();
        let response = harness
            .invoke(&artifact_service(), &store, &artifact_authority(), None)
            .unwrap();
        assert!(matches!(
            serde_json::from_slice::<ArtifactResponse>(&response).unwrap(),
            ArtifactResponse::Stored { reused: false, .. }
        ));

        let register = serde_json::to_vec(&ContextCommand::Register {
            resource_id: "skill:review".into(),
            kind: ContextResourceKind::Skill,
            source: "skills/review/SKILL.md".into(),
            scope: ContextScope::Workspace,
            content: b"review".to_vec(),
        })
        .unwrap();
        let response = harness
            .invoke(&context_service(), &register, &context_authority(), None)
            .unwrap();
        assert!(matches!(
            serde_json::from_slice::<ContextResponse>(&response).unwrap(),
            ContextResponse::Registered { .. }
        ));

        let objective = serde_json::to_vec(&PlanningCommand::CreateObjective {
            id: "objective-1".into(),
            title: "Use plugin-owned planning".into(),
            parent: None,
        })
        .unwrap();
        let response = harness
            .invoke(&planning_service(), &objective, &planning_authority(), None)
            .unwrap();
        assert!(matches!(
            serde_json::from_slice::<PlanningResponse>(&response).unwrap(),
            PlanningResponse::Objective { objective: Some(_) }
        ));
    }

    #[test]
    fn first_party_session_provider_is_replaceable_through_normal_resolution() {
        let alternate = serde_json::to_vec(&SessionResponse::Session { session: None }).unwrap();
        let alternate_factory = alternate.clone();
        let mut builder = HarnessBuilder::new();
        builder
            .add_embedded(session_manifest(), session_factory)
            .unwrap();
        builder
            .add_embedded(
                service_manifest(
                    "alternate-sessions",
                    session_service(),
                    200,
                    Authority::default(),
                ),
                move || Box::new(FixedResponse(alternate_factory.clone())),
            )
            .unwrap();
        let mut harness = builder.build().unwrap();
        harness.activate().unwrap();
        let input = serde_json::to_vec(&SessionCommand::Get { id: "x".into() }).unwrap();
        assert_eq!(
            harness
                .invoke(&session_service(), &input, &Authority::default(), None)
                .unwrap(),
            alternate
        );
    }

    #[test]
    fn first_party_context_provider_is_replaceable_through_normal_resolution() {
        let alternate = serde_json::to_vec(&ContextResponse::Resources {
            descriptors: Vec::new(),
        })
        .unwrap();
        let alternate_factory = alternate.clone();
        let mut builder = HarnessBuilder::new();
        builder
            .add_embedded(context_manifest(), context_factory)
            .unwrap();
        builder
            .add_embedded(
                service_manifest(
                    "alternate-context",
                    context_service(),
                    200,
                    Authority::default(),
                ),
                move || Box::new(FixedResponse(alternate_factory.clone())),
            )
            .unwrap();
        let mut harness = builder.build().unwrap();
        harness.activate().unwrap();
        let input = serde_json::to_vec(&ContextCommand::List).unwrap();
        assert_eq!(
            harness
                .invoke(&context_service(), &input, &Authority::default(), None)
                .unwrap(),
            alternate
        );
    }

    #[test]
    fn mock_qml_context_provider_contributes_through_the_same_service_contract() {
        let qml_descriptor = ContextDescriptor {
            resource_id: "qml:Main.qml".into(),
            revision: "qml-revision".into(),
            kind: ContextResourceKind::External,
            source: "Main.qml".into(),
            scope: ContextScope::Workspace,
            content_identity: "qml-revision".into(),
            estimated_bytes: 128,
        };
        let response = serde_json::to_vec(&ContextResponse::Resources {
            descriptors: vec![qml_descriptor.clone()],
        })
        .unwrap();
        let response_factory = response.clone();
        let mut builder = HarnessBuilder::new();
        builder
            .add_embedded(
                service_manifest(
                    "mock-qml-context",
                    context_service(),
                    200,
                    Authority::default(),
                ),
                move || Box::new(FixedResponse(response_factory.clone())),
            )
            .unwrap();
        let mut harness = builder.build().unwrap();
        harness.activate().unwrap();
        let input = serde_json::to_vec(&ContextCommand::List).unwrap();
        let output = harness
            .invoke(&context_service(), &input, &Authority::default(), None)
            .unwrap();
        assert_eq!(
            serde_json::from_slice::<ContextResponse>(&output).unwrap(),
            ContextResponse::Resources {
                descriptors: vec![qml_descriptor],
            }
        );
    }

    #[test]
    fn product_policy_can_replace_provider_without_kernel_changes() {
        let mut builder = HarnessBuilder::new();
        builder
            .add_embedded(manifest("first-party", 10), || Box::new(Echo(b"first")))
            .unwrap();
        builder
            .add_embedded(manifest("alternate", 20), || Box::new(Echo(b"alternate")))
            .unwrap();
        let mut harness = builder.build().unwrap();
        harness.activate().unwrap();

        assert_eq!(
            harness
                .invoke(&service(), b"", &Authority::default(), None)
                .unwrap(),
            b"alternate"
        );
        assert_eq!(
            harness
                .invoke(
                    &service(),
                    b"",
                    &Authority::default(),
                    Some(&plugin("first-party")),
                )
                .unwrap(),
            b"first"
        );
    }

    #[test]
    fn omitting_provider_removes_it_from_product_composition() {
        let mut builder = HarnessBuilder::new();
        builder
            .add_embedded(manifest("first-party", 10), || Box::new(Echo(b"first")))
            .unwrap();
        let mut harness = builder.build().unwrap();
        harness.activate().unwrap();

        assert_eq!(
            harness
                .invoke(&service(), b"", &Authority::default(), None)
                .unwrap(),
            b"first"
        );
        assert_eq!(
            harness.kernel().config().manifests().count(),
            1,
            "omitted plugins do not exist as kernel fallbacks"
        );
    }

    #[test]
    fn first_party_state_plugins_require_only_persistence_authority() {
        for authority in [
            session_authority(),
            artifact_authority(),
            context_authority(),
            planning_authority(),
        ] {
            assert!(authority.permits(&capability("kernel.persistence.schema")));
            assert!(authority.permits(&capability("kernel.persistence.read")));
            assert!(authority.permits(&capability("kernel.persistence.write")));
            assert!(!authority.permits(&capability("fs.write")));
        }
    }
}
