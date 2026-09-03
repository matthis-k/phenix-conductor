use crate::{implementation, job_component_id};
use phenix_core::{
    ComponentId, LayerResult, PluginHost, PluginInstance, PluginRuntimeProvider, ServiceId,
};

struct JobInstance {
    inner: Box<dyn PluginInstance>,
}

impl PluginInstance for JobInstance {
    fn start(&mut self, host: &PluginHost<'_>) -> Result<(), String> {
        self.inner.start(host)
    }

    fn runtime_provider(&mut self) -> Option<&mut dyn PluginRuntimeProvider> {
        self.inner.runtime_provider()
    }

    fn invoke(
        &mut self,
        service: &ServiceId,
        input: &[u8],
        host: &PluginHost<'_>,
    ) -> Result<Vec<u8>, String> {
        self.inner
            .invoke_component(&job_component_id(), service, input, host)
    }

    fn invoke_component(
        &mut self,
        component: &ComponentId,
        service: &ServiceId,
        input: &[u8],
        host: &PluginHost<'_>,
    ) -> Result<Vec<u8>, String> {
        self.inner.invoke_component(component, service, input, host)
    }

    fn invoke_layer(
        &mut self,
        service: &ServiceId,
        input: &[u8],
        host: &PluginHost<'_>,
    ) -> Result<LayerResult, String> {
        self.inner.invoke_layer(service, input, host)
    }

    fn stop(&mut self, host: &PluginHost<'_>) -> Result<(), String> {
        self.inner.stop(host)
    }
}

#[must_use]
pub fn job_factory() -> Box<dyn PluginInstance> {
    Box::new(JobInstance {
        inner: implementation::job_factory(),
    })
}
