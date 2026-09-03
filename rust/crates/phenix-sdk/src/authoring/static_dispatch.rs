use super::{
    EventContext, StaticComponentListener, StaticPluginComponents, StaticPluginResources,
};
use phenix_core::{
    Authority, ComponentId, EventBus, EventEnvelope, EventFailurePolicy, EventHandler,
    EventSubscription, EventTypeId, Exact, InterfaceId, LayerResult, PhenixValue, PluginContext,
    PluginHost, PluginId, PluginInstance, Project, ServiceId, SubscriptionId, SubscriptionSpec,
    ValueError,
};
use std::{
    error::Error,
    future::Future,
    sync::{Arc, Mutex, Weak},
    task::{Context, Poll, Wake, Waker},
    thread,
};

const STRUCTURAL_MISMATCH_EVENT: &str = "kernel.structural_value_mismatch";
const STRUCTURAL_MISMATCH_EVENT_VERSION: u32 = 1;

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
        Self: Sized + StaticComponentRuntimeDispatch + Send + 'static,
    {
        Box::new(StaticDispatchInstance(self))
    }

    fn default_plugin_instance() -> Box<dyn PluginInstance>
    where
        Self: Sized + StaticComponentRuntimeDispatch + Default + Send + 'static,
    {
        Self::default().into_plugin_instance()
    }
}

/// Live runtime callbacks generated from a component impl.
pub trait StaticComponentRuntimeDispatch {
    fn dispatch_runtime(
        &mut self,
        service: &ServiceId,
        input: &[u8],
        host: &PluginHost<'_>,
    ) -> Result<Vec<u8>, String>;

    fn dispatch_layer_runtime(
        &mut self,
        service: &ServiceId,
        input: &[u8],
        host: &PluginHost<'_>,
    ) -> Option<Result<LayerResult, String>>;

    #[doc(hidden)]
    fn dispatch_listener_runtime(
        &mut self,
        _listener: &str,
        _context: &EventContext,
        _payload: &[u8],
    ) -> Option<Result<(), Box<dyn Error + Send + Sync>>> {
        None
    }

    #[doc(hidden)]
    fn decode_projected_listener_runtime<T>(
        payload: &[u8],
    ) -> Result<T, Box<dyn Error + Send + Sync>>
    where
        Self: Sized,
        for<'value> T: TryFrom<Project<&'value PhenixValue>, Error = ValueError>,
    {
        decode_listener_runtime(payload, |value| T::try_from(Project(value)))
    }

    #[doc(hidden)]
    fn decode_exact_listener_runtime<T>(payload: &[u8]) -> Result<T, Box<dyn Error + Send + Sync>>
    where
        Self: Sized,
        for<'value> T: TryFrom<Exact<&'value PhenixValue>, Error = ValueError>,
    {
        decode_listener_runtime(payload, |value| T::try_from(Exact(value)))
    }
}

struct StaticDispatchInstance<T>(T);

impl<T> PluginInstance for StaticDispatchInstance<T>
where
    T: StaticComponentDispatch + StaticComponentRuntimeDispatch + Send + 'static,
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
        self.0.dispatch_runtime(service, input, host)
    }

    fn invoke_layer(
        &mut self,
        service: &ServiceId,
        input: &[u8],
        host: &PluginHost<'_>,
    ) -> Result<LayerResult, String> {
        self.0
            .dispatch_layer_runtime(service, input, host)
            .unwrap_or_else(|| Err(format!("unsupported component layer: {service}")))
    }
}

/// Kernel-scoped context for one layer invocation.
pub struct LayerContext<'host, 'runtime> {
    host: &'host PluginHost<'runtime>,
}

impl<'host, 'runtime> LayerContext<'host, 'runtime> {
    #[doc(hidden)]
    #[must_use]
    pub fn from_host(host: &'host PluginHost<'runtime>) -> Self {
        Self { host }
    }

    #[must_use]
    pub fn authority(&self) -> &Authority {
        self.host.authority()
    }

    #[must_use]
    pub fn graph_generation(&self) -> Option<&phenix_core::GraphGenerationId> {
        self.host.graph_generation()
    }

    pub fn delegate<T>(&self, request: &T) -> Result<LayerResult, String>
    where
        for<'value> PhenixValue: From<&'value T>,
    {
        let input = serde_json::to_vec(&PhenixValue::from(request))
            .map_err(|error| format!("layer request encoding failed: {error}"))?;
        self.continue_input(&input)
    }

    pub fn handle<T>(&self, response: &T) -> Result<LayerResult, String>
    where
        for<'value> PhenixValue: From<&'value T>,
    {
        let output = serde_json::to_vec(&PhenixValue::from(response))
            .map_err(|error| format!("layer response encoding failed: {error}"))?;
        Ok(LayerResult::Handled(output))
    }

    #[must_use]
    pub fn deny(message: impl Into<String>) -> LayerResult {
        LayerResult::Denied(message.into())
    }

    #[doc(hidden)]
    pub fn continue_input(&self, input: &[u8]) -> Result<LayerResult, String> {
        self.host
            .continue_service(input, self.host.authority())
            .map(LayerResult::Handled)
            .map_err(|error| error.to_string())
    }
}

#[doc(hidden)]
pub fn block_on_static<F: Future>(future: F) -> F::Output {
    struct ThreadWake(thread::Thread);

    impl Wake for ThreadWake {
        fn wake(self: Arc<Self>) {
            self.0.unpark();
        }

        fn wake_by_ref(self: &Arc<Self>) {
            self.0.unpark();
        }
    }

    let waker = Waker::from(Arc::new(ThreadWake(thread::current())));
    let mut context = Context::from_waker(&waker);
    let mut future = Box::pin(future);
    loop {
        match future.as_mut().poll(&mut context) {
            Poll::Ready(output) => return output,
            Poll::Pending => thread::park(),
        }
    }
}

#[doc(hidden)]
pub fn decode_projected_runtime<T>(
    host: &PluginHost<'_>,
    interface: &InterfaceId,
    input: &[u8],
) -> Result<T, String>
where
    for<'value> T: TryFrom<Project<&'value PhenixValue>, Error = ValueError>,
{
    PluginContext::new(host, (), (), ())
        .kernel
        .decode_projected(interface, input)
        .map_err(|error| error.to_string())
}

#[doc(hidden)]
pub fn decode_exact_runtime<T>(
    host: &PluginHost<'_>,
    interface: &InterfaceId,
    input: &[u8],
) -> Result<T, String>
where
    for<'value> T: TryFrom<Exact<&'value PhenixValue>, Error = ValueError>,
{
    PluginContext::new(host, (), (), ())
        .kernel
        .decode_exact(interface, input)
        .map_err(|error| error.to_string())
}

#[doc(hidden)]
pub fn encode_runtime<T>(host: &PluginHost<'_>, response: &T) -> Result<Vec<u8>, String>
where
    for<'value> PhenixValue: From<&'value T>,
{
    PluginContext::new(host, (), (), ())
        .kernel
        .encode_value(response)
        .map_err(|error| error.to_string())
}

fn decode_listener_runtime<T, F>(
    payload: &[u8],
    decode: F,
) -> Result<T, Box<dyn Error + Send + Sync>>
where
    F: FnOnce(&PhenixValue) -> Result<T, ValueError>,
{
    let value = serde_json::from_slice::<PhenixValue>(payload).map_err(|error| {
        Box::new(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("listener payload decoding failed: {error}"),
        )) as Box<dyn Error + Send + Sync>
    })?;
    decode(&value).map_err(|error| Box::new(error) as Box<dyn Error + Send + Sync>)
}

/// Author-free component routing generated for a stateful plugin struct.
pub trait StaticPluginComponentDispatch {
    fn dispatch_component(
        &mut self,
        component: &ComponentId,
        service: &ServiceId,
        input: &[u8],
        host: &PluginHost<'_>,
    ) -> Result<Vec<u8>, String>;

    fn dispatch_layer(
        &mut self,
        service: &ServiceId,
        _input: &[u8],
        _host: &PluginHost<'_>,
    ) -> Result<LayerResult, String> {
        Err(format!("unsupported static plugin layer: {service}"))
    }

    #[doc(hidden)]
    fn listener_subscriptions(
        _state: Weak<Mutex<Self>>,
        _host: &PluginHost<'_>,
    ) -> Vec<EventSubscription>
    where
        Self: Sized + Send + 'static,
    {
        Vec::new()
    }

    /// Adapt an already-constructed stateful plugin value to the kernel's erased runtime ABI.
    fn into_plugin_instance(self) -> Box<dyn PluginInstance>
    where
        Self: Sized + StaticPluginComponents + StaticPluginResources + Send + 'static,
    {
        Box::new(StaticPluginInstance::from_component_dispatch(self))
    }
}

pub type StaticPluginStart<T> = fn(&mut T, &PluginHost<'_>) -> Result<(), String>;
pub type StaticPluginStop<T> = fn(&mut T, &PluginHost<'_>) -> Result<(), String>;
pub type StaticPluginInvoke<T> =
    fn(&mut T, &ComponentId, &ServiceId, &[u8], &PluginHost<'_>) -> Result<Vec<u8>, String>;

/// Kernel adapter for an already-constructed Rust plugin value.
pub struct StaticPluginInstance<T> {
    plugin: Arc<Mutex<T>>,
    start: Option<StaticPluginStart<T>>,
    stop: Option<StaticPluginStop<T>>,
    invoke: StaticPluginInvoke<T>,
    listener_ids: Vec<SubscriptionId>,
}

impl<T> StaticPluginInstance<T>
where
    T: StaticPluginResources,
{
    #[must_use]
    pub fn new(plugin: T, invoke: StaticPluginInvoke<T>) -> Self {
        Self {
            plugin: Arc::new(Mutex::new(plugin)),
            start: None,
            stop: None,
            invoke,
            listener_ids: Vec::new(),
        }
    }
}

impl<T> StaticPluginInstance<T>
where
    T: StaticPluginResources + StaticPluginComponentDispatch,
{
    #[must_use]
    pub fn from_component_dispatch(plugin: T) -> Self {
        Self::new(plugin, T::dispatch_component)
    }
}

struct StaticListenerEventHandler<F> {
    owner: PluginId,
    listener: &'static str,
    handler: F,
}

impl<F> EventHandler for StaticListenerEventHandler<F>
where
    F: Fn(&EventEnvelope, &Authority) -> Result<(), Box<dyn Error + Send + Sync>> + Send + Sync,
{
    fn handle(&self, event: &EventEnvelope, authority: &Authority) -> Result<(), String> {
        (self.handler)(event, authority).map_err(|error| error.to_string())
    }

    fn handle_with_bus(
        &self,
        bus: &EventBus,
        event: &EventEnvelope,
        authority: &Authority,
    ) -> Result<(), String> {
        match (self.handler)(event, authority) {
            Ok(()) => Ok(()),
            Err(error) => {
                if let Some(value_error) = error.downcast_ref::<ValueError>() {
                    report_listener_mismatch(
                        bus,
                        &self.owner,
                        self.listener,
                        event,
                        authority,
                        value_error,
                    );
                }
                Err(error.to_string())
            }
        }
    }
}

fn report_listener_mismatch(
    events: &EventBus,
    owner: &PluginId,
    listener: &str,
    source: &EventEnvelope,
    authority: &Authority,
    error: &ValueError,
) {
    let Ok(payload) = serde_json::to_vec(&serde_json::json!({
        "event": source.event_type.as_str(),
        "listener": listener,
        "direction": "listener",
        "error": error.to_string(),
    })) else {
        return;
    };
    let diagnostic = EventEnvelope {
        event_type: EventTypeId::parse(STRUCTURAL_MISMATCH_EVENT)
            .expect("static structural mismatch event type is valid"),
        version: STRUCTURAL_MISMATCH_EVENT_VERSION,
        emitter: owner.clone(),
        causality_id: source.causality_id,
        kernel_policy_revision: source.kernel_policy_revision,
        payload,
    };
    let _ = events.dispatch(&diagnostic, authority);
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
        match Arc::try_unwrap(self.plugin) {
            Ok(plugin) => plugin
                .into_inner()
                .unwrap_or_else(|error| error.into_inner()),
            Err(_) => panic!("static plugin state still has live strong references"),
        }
    }

    #[doc(hidden)]
    pub fn listener_subscription<F>(
        owner: PluginId,
        component: &ComponentId,
        listener: &StaticComponentListener,
        maximum_authority: Authority,
        handler: F,
    ) -> EventSubscription
    where
        F: Fn(&EventEnvelope, &Authority) -> Result<(), Box<dyn Error + Send + Sync>>
            + Send
            + Sync
            + 'static,
    {
        let id = SubscriptionId::parse(format!(
            "{}/listener/{}/{}",
            owner.as_str(),
            component.as_str(),
            listener.method
        ))
        .expect("generated stateful listener subscription id is valid");
        let diagnostic_owner = owner.clone();
        EventSubscription {
            spec: SubscriptionSpec {
                id,
                owner,
                event_type: listener.event.clone(),
                event_version: 1,
                dependencies: Vec::new(),
                failure_policy: EventFailurePolicy::Warn,
                required_authority: listener.required_authority.clone(),
                maximum_authority,
                kernel_policy_revision: 0,
            },
            handler: Arc::new(StaticListenerEventHandler {
                owner: diagnostic_owner,
                listener: listener.method,
                handler,
            }),
        }
    }
}

fn register_resources<T: StaticPluginResources>(host: &PluginHost<'_>) -> Result<(), String> {
    T::register_resource_schemas(host).map_err(|error| error.to_string())
}

impl<T> PluginInstance for StaticPluginInstance<T>
where
    T: StaticPluginComponents + StaticPluginResources + StaticPluginComponentDispatch + Send + 'static,
{
    fn start(&mut self, host: &PluginHost<'_>) -> Result<(), String> {
        register_resources::<T>(host)?;
        let subscriptions = T::listener_subscriptions(Arc::downgrade(&self.plugin), host);
        self.listener_ids = host
            .install_event_subscriptions(subscriptions)
            .map_err(|error| error.to_string())?;

        let result = match self.start {
            Some(start) => match self.plugin.lock() {
                Ok(mut plugin) => start(&mut plugin, host),
                Err(_) => Err("static plugin state lock poisoned".to_owned()),
            },
            None => Ok(()),
        };
        if result.is_err() {
            let listener_ids = std::mem::take(&mut self.listener_ids);
            let _ = host.remove_event_subscriptions(listener_ids);
        }
        result
    }

    fn invoke(
        &mut self,
        service: &ServiceId,
        input: &[u8],
        host: &PluginHost<'_>,
    ) -> Result<Vec<u8>, String> {
        let mut matches = T::components().into_iter().filter(|component| {
            component.exports().iter().any(|export| {
                export.terminal && export.interface.as_str() == service.as_str()
            })
        });
        let component = matches
            .next()
            .ok_or_else(|| format!("unsupported static plugin service: {service}"))?;
        if matches.next().is_some() {
            return Err(format!("ambiguous static plugin service: {service}"));
        }
        self.invoke_component(&component.id, service, input, host)
    }

    fn invoke_component(
        &mut self,
        component: &ComponentId,
        service: &ServiceId,
        input: &[u8],
        host: &PluginHost<'_>,
    ) -> Result<Vec<u8>, String> {
        let mut plugin = self
            .plugin
            .lock()
            .map_err(|_| "static plugin state lock poisoned".to_owned())?;
        (self.invoke)(&mut plugin, component, service, input, host)
    }

    fn invoke_layer(
        &mut self,
        service: &ServiceId,
        input: &[u8],
        host: &PluginHost<'_>,
    ) -> Result<LayerResult, String> {
        let mut plugin = self
            .plugin
            .lock()
            .map_err(|_| "static plugin state lock poisoned".to_owned())?;
        T::dispatch_layer(&mut plugin, service, input, host)
    }

    fn stop(&mut self, host: &PluginHost<'_>) -> Result<(), String> {
        let stop_result = match self.stop {
            Some(stop) => match self.plugin.lock() {
                Ok(mut plugin) => stop(&mut plugin, host),
                Err(_) => Err("static plugin state lock poisoned".to_owned()),
            },
            None => Ok(()),
        };
        let listener_ids = std::mem::take(&mut self.listener_ids);
        let remove_result = host
            .remove_event_subscriptions(listener_ids)
            .map_err(|error| error.to_string());
        match (stop_result, remove_result) {
            (Err(stop), Err(remove)) => Err(format!("{stop}; listener cleanup failed: {remove}")),
            (Err(stop), Ok(())) => Err(stop),
            (Ok(()), Err(remove)) => Err(remove),
            (Ok(()), Ok(())) => Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{StaticComponentDescriptor, StaticResourceDescriptor};
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

    impl StaticComponentRuntimeDispatch for Component {
        fn dispatch_runtime(
            &mut self,
            _service: &ServiceId,
            _input: &[u8],
            _host: &PluginHost<'_>,
        ) -> Result<Vec<u8>, String> {
            Ok(Vec::new())
        }

        fn dispatch_layer_runtime(
            &mut self,
            _service: &ServiceId,
            _input: &[u8],
            _host: &PluginHost<'_>,
        ) -> Option<Result<LayerResult, String>> {
            None
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

    impl StaticPluginComponents for StatefulPlugin {
        fn components() -> Vec<StaticComponentDescriptor> {
            Vec::new()
        }
    }

    impl StaticPluginResources for StatefulPlugin {
        fn resources() -> Vec<StaticResourceDescriptor> {
            Vec::new()
        }
    }

    impl StaticPluginComponentDispatch for StatefulPlugin {
        fn dispatch_component(
            &mut self,
            _component: &ComponentId,
            _service: &ServiceId,
            input: &[u8],
            _host: &PluginHost<'_>,
        ) -> Result<Vec<u8>, String> {
            self.calls += 1;
            Ok(input.to_vec())
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
    fn generated_stateful_plugin_dispatch_adapts_to_plugin_instance() {
        let instance: Box<dyn PluginInstance> = StatefulPlugin { calls: 7 }.into_plugin_instance();
        drop(instance);
    }

    #[test]
    fn stateful_plugin_adapter_preserves_non_default_state_and_callbacks() {
        let instance = StaticPluginInstance::new(StatefulPlugin { calls: 7 }, invoke)
            .with_start(start)
            .with_stop(stop);

        assert_eq!(instance.plugin.lock().unwrap().calls, 7);
        assert!(instance.start.is_some());
        assert!(instance.stop.is_some());
    }
}
