use phenix_kernel::{
    Authority, Kernel, KernelConfig, KernelError, PluginExecution, PluginId, PluginInstance,
    PluginManifest,
};
use phenix_plugin_suite::{repository_worker_factory, repository_worker_manifest};
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
    use phenix_kernel::{ServiceContribution, ServiceId};
    use phenix_plugin_suite::{repository_work_queue_service, RepositoryWorkSnapshot};

    fn plugin(value: &str) -> PluginId {
        PluginId::parse(value).unwrap()
    }

    fn service() -> ServiceId {
        ServiceId::parse("fixture.echo@1").unwrap()
    }

    fn manifest(id: &str, priority: i32) -> PluginManifest {
        PluginManifest {
            id: plugin(id),
            version: 1,
            execution: PluginExecution::Embedded,
            dependencies: Vec::new(),
            services: vec![ServiceContribution {
                service: service(),
                priority,
                required_authority: Authority::default(),
            }],
            resource_namespaces: Vec::new(),
            maximum_authority: Authority::default(),
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
            _authority: &Authority,
        ) -> Result<Vec<u8>, String> {
            Ok(self.0.to_vec())
        }
    }

    #[test]
    fn kernel_only_harness_has_no_userspace_plugins() {
        let mut harness = PhenixHarness::kernel_only();
        harness.activate().unwrap();
        assert_eq!(harness.kernel().config().manifests().count(), 0);
    }

    #[test]
    fn default_harness_loads_first_party_suite_through_kernel_contracts() {
        let mut harness = PhenixHarness::default_suite().unwrap();
        harness.activate().unwrap();
        assert_eq!(harness.kernel().config().manifests().count(), 1);

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
}
