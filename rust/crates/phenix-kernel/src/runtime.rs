use crate::{
    Authority, EventBus, KernelConfig, KernelError, KernelEvent, PluginExecution, PluginId,
    PluginManifest, ProviderBinding, ServiceId, TaskRuntime,
};
use std::{collections::BTreeMap, sync::Arc};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PluginState {
    Registered,
    Active,
    Stopped,
}

pub struct PluginHost<'a> {
    config: &'a KernelConfig,
    plugin: &'a PluginId,
    authority: &'a Authority,
}

impl<'a> PluginHost<'a> {
    pub fn plugin(&self) -> &PluginId {
        self.plugin
    }

    pub fn authority(&self) -> &Authority {
        self.authority
    }

    pub fn resolve_service(
        &self,
        service: &ServiceId,
        binding: Option<&PluginId>,
    ) -> Result<ProviderBinding, KernelError> {
        self.config.resolve(service, self.authority, binding)
    }
}

pub trait PluginInstance: Send {
    fn start(&mut self, host: &PluginHost<'_>) -> Result<(), String>;

    fn invoke(
        &mut self,
        _service: &ServiceId,
        _input: &[u8],
        _authority: &Authority,
    ) -> Result<Vec<u8>, String> {
        Err("service invocation is not implemented".into())
    }

    fn stop(&mut self) -> Result<(), String> {
        Ok(())
    }
}

type EmbeddedFactory = Arc<dyn Fn() -> Box<dyn PluginInstance> + Send + Sync>;
type ExternalFactory =
    Arc<dyn Fn(&PluginManifest) -> Result<Box<dyn PluginInstance>, String> + Send + Sync>;

pub struct Kernel {
    config: KernelConfig,
    states: BTreeMap<PluginId, PluginState>,
    embedded_factories: BTreeMap<PluginId, EmbeddedFactory>,
    external_factories: BTreeMap<PluginId, ExternalFactory>,
    instances: BTreeMap<PluginId, Box<dyn PluginInstance>>,
    events: Arc<EventBus>,
    tasks: TaskRuntime,
}

impl Kernel {
    pub fn new(config: KernelConfig) -> Self {
        let states = config
            .manifests()
            .map(|manifest| (manifest.id.clone(), PluginState::Registered))
            .collect();
        Self {
            config,
            states,
            embedded_factories: BTreeMap::new(),
            external_factories: BTreeMap::new(),
            instances: BTreeMap::new(),
            events: Arc::new(EventBus::default()),
            tasks: TaskRuntime::default(),
        }
    }

    pub fn kernel_only() -> Self {
        Self::new(KernelConfig::empty())
    }

    pub fn config(&self) -> &KernelConfig {
        &self.config
    }

    pub fn events(&self) -> Arc<EventBus> {
        Arc::clone(&self.events)
    }

    pub fn tasks(&self) -> &TaskRuntime {
        &self.tasks
    }

    pub fn state(&self, plugin: &PluginId) -> Option<PluginState> {
        self.states.get(plugin).copied()
    }

    pub fn register_embedded_factory<F>(
        &mut self,
        plugin: PluginId,
        factory: F,
    ) -> Result<(), KernelError>
    where
        F: Fn() -> Box<dyn PluginInstance> + Send + Sync + 'static,
    {
        let manifest = self
            .config
            .manifest(&plugin)
            .ok_or_else(|| KernelError::UnknownPlugin(plugin.clone()))?;
        if !matches!(manifest.execution, PluginExecution::Embedded) {
            return Err(KernelError::WrongExecutionKind(plugin));
        }
        self.embedded_factories.insert(plugin, Arc::new(factory));
        Ok(())
    }

    pub fn register_external_factory<F>(
        &mut self,
        plugin: PluginId,
        factory: F,
    ) -> Result<(), KernelError>
    where
        F: Fn(&PluginManifest) -> Result<Box<dyn PluginInstance>, String> + Send + Sync + 'static,
    {
        let manifest = self
            .config
            .manifest(&plugin)
            .ok_or_else(|| KernelError::UnknownPlugin(plugin.clone()))?;
        if !matches!(manifest.execution, PluginExecution::External { .. }) {
            return Err(KernelError::WrongExecutionKind(plugin));
        }
        self.external_factories.insert(plugin, Arc::new(factory));
        Ok(())
    }

    pub fn activate_all(&mut self) -> Result<(), KernelError> {
        for plugin in self.config.activation_order().to_vec() {
            self.activate(&plugin)?;
        }
        Ok(())
    }

    fn activate(&mut self, plugin: &PluginId) -> Result<(), KernelError> {
        if self.state(plugin) == Some(PluginState::Active) {
            return Ok(());
        }
        let manifest = self
            .config
            .manifest(plugin)
            .ok_or_else(|| KernelError::UnknownPlugin(plugin.clone()))?
            .clone();

        match manifest.execution {
            PluginExecution::ResourceOnly => {}
            PluginExecution::Embedded => {
                let factory = self
                    .embedded_factories
                    .get(plugin)
                    .ok_or_else(|| KernelError::EmbeddedFactoryMissing(plugin.clone()))?;
                let instance = factory();
                self.start_instance(plugin, &manifest, instance)?;
            }
            PluginExecution::External { .. } => {
                let factory = self
                    .external_factories
                    .get(plugin)
                    .ok_or_else(|| KernelError::ExternalHostUnavailable(plugin.clone()))?;
                let instance = factory(&manifest).map_err(|message| KernelError::PluginStart {
                    plugin: plugin.clone(),
                    message,
                })?;
                self.start_instance(plugin, &manifest, instance)?;
            }
        }

        self.states.insert(plugin.clone(), PluginState::Active);
        self.events
            .publish(KernelEvent::PluginActivated(plugin.clone()));
        Ok(())
    }

    fn start_instance(
        &mut self,
        plugin: &PluginId,
        manifest: &PluginManifest,
        mut instance: Box<dyn PluginInstance>,
    ) -> Result<(), KernelError> {
        let host = PluginHost {
            config: &self.config,
            plugin,
            authority: &manifest.maximum_authority,
        };
        instance
            .start(&host)
            .map_err(|message| KernelError::PluginStart {
                plugin: plugin.clone(),
                message,
            })?;
        self.instances.insert(plugin.clone(), instance);
        Ok(())
    }

    pub fn invoke(
        &mut self,
        service: &ServiceId,
        input: &[u8],
        caller_authority: &Authority,
        binding: Option<&PluginId>,
    ) -> Result<Vec<u8>, KernelError> {
        let provider = self.config.resolve(service, caller_authority, binding)?;
        if self.state(&provider.plugin) != Some(PluginState::Active) {
            return Err(KernelError::PluginNotActive(provider.plugin));
        }
        let provider_manifest = self
            .config
            .manifest(&provider.plugin)
            .expect("resolved providers are registered");
        let effective_authority = caller_authority.attenuate(&provider_manifest.maximum_authority);
        let instance = self
            .instances
            .get_mut(&provider.plugin)
            .ok_or_else(|| KernelError::WrongExecutionKind(provider.plugin.clone()))?;
        instance
            .invoke(service, input, &effective_authority)
            .map_err(|message| KernelError::ServiceInvoke {
                plugin: provider.plugin,
                service: service.clone(),
                message,
            })
    }

    pub fn stop(&mut self, plugin: &PluginId) -> Result<(), KernelError> {
        if let Some(mut instance) = self.instances.remove(plugin) {
            instance
                .stop()
                .map_err(|message| KernelError::PluginStart {
                    plugin: plugin.clone(),
                    message,
                })?;
        }
        let state = self
            .states
            .get_mut(plugin)
            .ok_or_else(|| KernelError::UnknownPlugin(plugin.clone()))?;
        *state = PluginState::Stopped;
        self.events
            .publish(KernelEvent::PluginStopped(plugin.clone()));
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CapabilityId, PluginManifest, ResourceNamespace, ServiceContribution};
    use std::sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    };

    fn plugin(value: &str) -> PluginId {
        PluginId::parse(value).unwrap()
    }

    fn capability(value: &str) -> CapabilityId {
        CapabilityId::parse(value).unwrap()
    }

    fn service(value: &str) -> ServiceId {
        ServiceId::parse(value).unwrap()
    }

    struct MarkerPlugin(Arc<AtomicBool>);

    impl PluginInstance for MarkerPlugin {
        fn start(&mut self, host: &PluginHost<'_>) -> Result<(), String> {
            assert_eq!(host.plugin().as_str(), "embedded");
            self.0.store(true, Ordering::Release);
            Ok(())
        }
    }

    struct EchoPlugin;

    impl PluginInstance for EchoPlugin {
        fn start(&mut self, _host: &PluginHost<'_>) -> Result<(), String> {
            Ok(())
        }

        fn invoke(
            &mut self,
            _service: &ServiceId,
            input: &[u8],
            authority: &Authority,
        ) -> Result<Vec<u8>, String> {
            if authority.permits(&capability("fs.write")) {
                return Err("provider regained caller write authority".into());
            }
            Ok(input.to_vec())
        }
    }

    #[test]
    fn kernel_only_boots_without_agent_domain_services() {
        let mut kernel = Kernel::kernel_only();
        kernel.activate_all().unwrap();
        assert_eq!(kernel.config().manifests().count(), 0);
    }

    #[test]
    fn embedded_and_resource_only_plugins_share_lifecycle_contract() {
        let marker = Arc::new(AtomicBool::new(false));
        let embedded = PluginManifest {
            id: plugin("embedded"),
            version: 1,
            execution: PluginExecution::Embedded,
            dependencies: Vec::new(),
            services: Vec::new(),
            resource_namespaces: Vec::new(),
            maximum_authority: Authority::default(),
        };
        let mut resources = PluginManifest::resource_only(plugin("resources"));
        resources
            .resource_namespaces
            .push(ResourceNamespace::parse("resources.static").unwrap());

        let mut kernel = Kernel::new(KernelConfig::new([resources, embedded]).unwrap());
        let marker_for_factory = Arc::clone(&marker);
        kernel
            .register_embedded_factory(plugin("embedded"), move || {
                Box::new(MarkerPlugin(Arc::clone(&marker_for_factory)))
            })
            .unwrap();

        kernel.activate_all().unwrap();

        assert!(marker.load(Ordering::Acquire));
        assert_eq!(kernel.state(&plugin("embedded")), Some(PluginState::Active));
        assert_eq!(
            kernel.state(&plugin("resources")),
            Some(PluginState::Active)
        );
        assert_eq!(
            kernel
                .config()
                .resource_owner(&ResourceNamespace::parse("resources.static").unwrap()),
            Some(&plugin("resources"))
        );
    }

    #[test]
    fn invocation_uses_caller_authority_attenuated_by_provider_grant() {
        let read = capability("fs.read");
        let write = capability("fs.write");
        let provider = PluginManifest {
            id: plugin("echo"),
            version: 1,
            execution: PluginExecution::Embedded,
            dependencies: Vec::new(),
            services: vec![ServiceContribution {
                service: service("echo@1"),
                priority: 1,
                required_authority: Authority::new([read.clone()]),
            }],
            resource_namespaces: Vec::new(),
            maximum_authority: Authority::new([read.clone()]),
        };
        let mut kernel = Kernel::new(KernelConfig::new([provider]).unwrap());
        kernel
            .register_embedded_factory(plugin("echo"), || Box::new(EchoPlugin))
            .unwrap();
        kernel.activate_all().unwrap();

        let output = kernel
            .invoke(
                &service("echo@1"),
                b"hello",
                &Authority::new([read, write]),
                None,
            )
            .unwrap();
        assert_eq!(output, b"hello");
    }
}
