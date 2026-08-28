#![forbid(unsafe_code)]

mod component;
mod configuration;
#[cfg(test)]
mod generation_regression;
mod implementation {
    include!("implementation.rs");
}

pub use component::*;
pub use configuration::{
    execution_configuration_service, AgentDefinition, CallablePolicy,
    ExecutionConfigurationCommand, ExecutionConfigurationResponse, OrchestrationDefinition,
    OrchestrationNode, EXECUTION_CONFIGURATION_SERVICE,
};
pub use implementation::{
    execution_service, CallableRecord, ExecutionAuthority, ExecutionCommand, ExecutionRecord,
    ExecutionResponse, ExecutionState, WorkerTaskRecord, WorkerTaskState, EXECUTION_SERVICE,
};

use phenix_core::{
    Authority, PluginHost, PluginInstance, PluginManifest, ServiceContribution, ServiceId,
};

#[must_use]
pub fn execution_manifest(maximum_authority: Authority) -> PluginManifest {
    let mut manifest = implementation::execution_manifest(maximum_authority);
    manifest.services.push(ServiceContribution {
        role: phenix_core::ServiceRole::Terminal,
        service: configuration::execution_configuration_service(),
        priority: 100,
        required_authority: Authority::default(),
    });
    manifest
        .resource_namespaces
        .push(configuration::execution_configuration_namespace());
    manifest
}

#[must_use]
pub fn execution_factory() -> Box<dyn PluginInstance> {
    Box::new(ExecutionPackagePlugin {
        execution: implementation::execution_factory(),
        configuration: configuration::configuration_factory(),
    })
}

struct ExecutionPackagePlugin {
    execution: Box<dyn PluginInstance>,
    configuration: Box<dyn PluginInstance>,
}

impl PluginInstance for ExecutionPackagePlugin {
    fn start(&mut self, host: &PluginHost<'_>) -> Result<(), String> {
        self.execution.start(host)?;
        self.configuration.start(host)
    }

    fn invoke(
        &mut self,
        service: &ServiceId,
        input: &[u8],
        host: &PluginHost<'_>,
    ) -> Result<Vec<u8>, String> {
        if service == &configuration::execution_configuration_service() {
            self.configuration.invoke(service, input, host)
        } else {
            self.execution.invoke(service, input, host)
        }
    }

    fn stop(&mut self) -> Result<(), String> {
        self.configuration.stop()?;
        self.execution.stop()
    }
}
