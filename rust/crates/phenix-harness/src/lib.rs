use phenix_kernel::{
    Authority, Kernel, KernelConfig, KernelError, PluginExecution, PluginId, PluginInstance,
    PluginManifest,
};
use phenix_plugin_suite::{
    artifact_factory, artifact_manifest, repository_worker_factory, repository_worker_manifest,
    session_factory, session_manifest,
};
use std::{collections::BTreeMap, sync::Arc};

type EmbeddedFactory = Arc<dyn Fn() -> Box<dyn PluginInstance> + Send + Sync>;

#[derive(Default)]
pub struct HarnessBuilder {
    manifests: Vec<PluginManifest>,
    embedded_factories: BTreeMap<PluginId, EmbeddedFactory>,
}

impl HarnessBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_default_suite() -> Result<Self, KernelError> {
        let mut builder = Self::new();
        builder.add_embedded(repository_worker_manifest(), repository_worker_factory)?;
        builder.add_embedded(session_manifest(), session_factory)?;
        builder.add_embedded(artifact_manifest(), artifact_factory)?;
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

    pub fn build(self) -> Result<PhenixHarness, KernelError> {
        let config = KernelConfig::new(self.manifests)?;
        let mut kernel = Kernel::new(config);
        for (plugin, factory) in self.embedded_factories {
            kernel.register_embedded_factory(plugin, move || factory())?;
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
        artifact_manifest, artifact_service, repository_work_queue_service, session_manifest,
        session_service, ArtifactCommand, ArtifactResponse, RepositoryWorkSnapshot, SessionCommand,
        SessionResponse,
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
    }

    #[test]
    fn default_harness_loads_first_party_suite_through_kernel_contracts() {
        let mut harness = PhenixHarness::default_suite().unwrap();
        harness.activate().unwrap();
        assert_eq!(harness.kernel().config().manifests().count(), 3);

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
            id: "read:README.md".into(),
            content_identity: "sha256:readme".into(),
            content: b"readme".to_vec(),
        })
        .unwrap();
        let response = harness
            .invoke(&artifact_service(), &store, &artifact_authority(), None)
            .unwrap();
        assert!(matches!(
            serde_json::from_slice::<ArtifactResponse>(&response).unwrap(),
            ArtifactResponse::Stored { reused: false, .. }
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
                move || {
                    Box::new(Echo(Box::leak(
                        alternate_factory.clone().into_boxed_slice(),
                    )))
                },
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
        for authority in [session_authority(), artifact_authority()] {
            assert!(authority.permits(&capability("kernel.persistence.schema")));
            assert!(authority.permits(&capability("kernel.persistence.read")));
            assert!(authority.permits(&capability("kernel.persistence.write")));
            assert!(!authority.permits(&capability("fs.write")));
        }
    }
}
