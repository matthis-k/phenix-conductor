use super::StaticPluginResources;
use phenix_core::{ComponentId, PluginHost, PluginInstance, ServiceId};

/// Erased runtime dispatch generated from a stateful component's typed exports.
pub trait StaticComponentDispatch {
    fn dispatch(
        &mut self,
        service: &ServiceId,
        input: &[u8],
        host: &PluginHost<'_>,
    ) -> Result<Vec<u8>, String>;

    fn into_plugin_instance(self) -> Box<dyn PluginInstance>
    where
        Self: Sized + Send + 'static,
    {
        Box::new(StaticDispatchInstance(self))
    }

    fn default_plugin_instance() -> Box<dyn PluginInstance>
    where
        Self: Sized + Default + Send + 'static,
    {
        Self::default().into_plugin_instance()
    }
}

struct StaticDispatchInstance<T>(T);

impl<T> PluginInstance for StaticDispatchInstance<T>
where
    T: StaticComponentDispatch + Send + 'static,
{
    fn start(&mut self, _host: &PluginHost<'_>) -> Result<(), String> {
        Ok(())
    }

    fn invoke(
        &mut self,
        service: &ServiceId,
        input: &[u8],
        host: &PluginHost<'_>,
    ) -> Result<Vec<u8>, String> {
        self.0.dispatch(service, input, host)
    }
}

pub type StaticPluginStart<T> = fn(&mut T, &PluginHost<'_>) -> Result<(), String>;
pub type StaticPluginStop<T> = fn(&mut T, &PluginHost<'_>) -> Result<(), String>;
pub type StaticPluginInvoke<T> =
    fn(&mut T, &ComponentId, &ServiceId, &[u8], &PluginHost<'_>) -> Result<Vec<u8>, String>;

/// Kernel adapter for an already-constructed Rust plugin value.
///
/// The authoring macro supplies the callbacks. Construction remains separate so
/// plugins with ordinary non-`Default` state can use their normal constructor or
/// generated builder without changing runtime dispatch semantics.
pub struct StaticPluginInstance<T> {
    plugin: T,
    start: Option<StaticPluginStart<T>>,
    stop: Option<StaticPluginStop<T>>,
    invoke: StaticPluginInvoke<T>,
}

impl<T> StaticPluginInstance<T>
where
    T: StaticPluginResources,
{
    #[must_use]
    pub fn new(plugin: T, invoke: StaticPluginInvoke<T>) -> Self {
        Self {
            plugin,
            start: None,
            stop: None,
            invoke,
        }
    }
}

impl<T> StaticPluginInstance<T> {
    #[must_use]
    pub fn with_start(mut self, start: StaticPluginStart<T>) -> Self {
        self.start = Some(start);
        self
    }

    #[must_use]
    pub fn with_stop(mut self, stop: StaticPluginStop<T>) -> Self {
        self.stop = Some(stop);
        self
    }

    #[must_use]
    pub fn into_inner(self) -> T {
        self.plugin
    }
}

fn register_resources<T: StaticPluginResources>(host: &PluginHost<'_>) -> Result<(), String> {
    T::register_resource_schemas(host).map_err(|error| error.to_string())
}

impl<T> PluginInstance for StaticPluginInstance<T>
where
    T: StaticPluginResources + Send + 'static,
{
    fn start(&mut self, host: &PluginHost<'_>) -> Result<(), String> {
        register_resources::<T>(host)?;
        match self.start {
            Some(start) => start(&mut self.plugin, host),
            None => Ok(()),
        }
    }

    fn invoke_component(
        &mut self,
        component: &ComponentId,
        service: &ServiceId,
        input: &[u8],
        host: &PluginHost<'_>,
    ) -> Result<Vec<u8>, String> {
        (self.invoke)(&mut self.plugin, component, service, input, host)
    }

    fn stop(&mut self, host: &PluginHost<'_>) -> Result<(), String> {
        match self.stop {
            Some(stop) => stop(&mut self.plugin, host),
            None => Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::StaticResourceDescriptor;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[derive(Default)]
    struct Component;

    impl StaticComponentDispatch for Component {
        fn dispatch(
            &mut self,
            _service: &ServiceId,
            _input: &[u8],
            _host: &PluginHost<'_>,
        ) -> Result<Vec<u8>, String> {
            Ok(Vec::new())
        }
    }

    #[test]
    fn generated_component_dispatch_adapts_to_plugin_instance() {
        let instance: Box<dyn PluginInstance> = Component.into_plugin_instance();
        drop(instance);
    }

    #[test]
    fn default_component_builds_a_kernel_plugin_instance() {
        let instance: Box<dyn PluginInstance> = Component::default_plugin_instance();
        drop(instance);
    }

    struct StatefulPlugin {
        calls: usize,
    }

    impl StaticPluginResources for StatefulPlugin {
        fn resources() -> Vec<StaticResourceDescriptor> {
            Vec::new()
        }
    }

    static STARTS: AtomicUsize = AtomicUsize::new(0);
    static STOPS: AtomicUsize = AtomicUsize::new(0);

    fn start(_plugin: &mut StatefulPlugin, _host: &PluginHost<'_>) -> Result<(), String> {
        STARTS.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }

    fn stop(_plugin: &mut StatefulPlugin, _host: &PluginHost<'_>) -> Result<(), String> {
        STOPS.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }

    fn invoke(
        plugin: &mut StatefulPlugin,
        _component: &ComponentId,
        _service: &ServiceId,
        input: &[u8],
        _host: &PluginHost<'_>,
    ) -> Result<Vec<u8>, String> {
        plugin.calls += 1;
        Ok(input.to_vec())
    }

    #[test]
    fn stateful_plugin_adapter_preserves_non_default_state_and_callbacks() {
        let instance = StaticPluginInstance::new(StatefulPlugin { calls: 7 }, invoke)
            .with_start(start)
            .with_stop(stop);

        assert_eq!(instance.plugin.calls, 7);
        assert!(instance.start.is_some());
        assert!(instance.stop.is_some());
    }
}
