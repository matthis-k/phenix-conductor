use crate::{
    Authority, ComponentId, ComponentInterface, ComponentInvocationError, DurableSchema,
    EventDispatchReport, EventError, EventTypeId, GraphGenerationId, KernelError,
    NamespaceTransaction, PluginHost, PluginId, ResourceNamespace, SchemaMigration, ServiceId,
    TaskHandle, TransactionOp,
};
use std::marker::PhantomData;

/// Runtime view passed to plugin code.
///
/// The context separates generic kernel access, imported SDK contracts, current
/// plugin data, and current call data.
pub struct PluginContext<'host, 'runtime, Sdk, Settings = (), State = ()> {
    pub kernel: KernelAccess<'host, 'runtime>,
    pub sdk: Sdk,
    pub plugin: CurrentPlugin<'host, Settings, State>,
    pub call: CallContext<'host>,
}

impl<'host, 'runtime, Sdk, Settings, State> PluginContext<'host, 'runtime, Sdk, Settings, State> {
    pub fn new(
        host: &'host PluginHost<'runtime>,
        sdk: Sdk,
        settings: Settings,
        state: State,
    ) -> Self {
        Self {
            kernel: KernelAccess::new(host),
            sdk,
            plugin: CurrentPlugin {
                id: host.plugin(),
                settings,
                state,
            },
            call: CallContext {
                authority: host.authority(),
                graph_generation: host.graph_generation(),
            },
        }
    }
}

/// Data scoped to the current plugin instance.
pub struct CurrentPlugin<'host, Settings, State> {
    pub id: &'host PluginId,
    pub settings: Settings,
    pub state: State,
}

/// Data scoped to the current kernel-mediated call.
pub struct CallContext<'host> {
    pub authority: &'host Authority,
    pub graph_generation: Option<&'host GraphGenerationId>,
}

/// Scoped access to generic kernel mechanisms.
///
/// This wraps `PluginHost`; plugin business logic does not need the host itself.
#[derive(Clone, Copy)]
pub struct KernelAccess<'host, 'runtime> {
    host: &'host PluginHost<'runtime>,
}

impl<'host, 'runtime> KernelAccess<'host, 'runtime> {
    fn new(host: &'host PluginHost<'runtime>) -> Self {
        Self { host }
    }

    #[doc(hidden)]
    pub fn invoke_service_abi(
        &self,
        service: &ServiceId,
        input: &[u8],
        requested_authority: &Authority,
        binding: Option<&PluginId>,
    ) -> Result<Vec<u8>, KernelError> {
        self.host
            .invoke_service_abi(service, input, requested_authority, binding)
    }

    pub fn continue_service(
        &self,
        input: &[u8],
        requested_authority: &Authority,
    ) -> Result<Vec<u8>, KernelError> {
        self.host.continue_service(input, requested_authority)
    }

    pub fn spawn_task<T, F>(&self, requested_authority: &Authority, worker: F) -> TaskHandle<T>
    where
        T: Send + 'static,
        F: FnOnce(crate::CancellationToken) -> T + Send + 'static,
    {
        self.host.spawn_task(requested_authority, worker)
    }

    pub fn dispatch_event(
        &self,
        event_type: EventTypeId,
        version: u32,
        causality_id: u64,
        kernel_policy_revision: u64,
        payload: Vec<u8>,
    ) -> Result<EventDispatchReport, EventError> {
        self.host.dispatch_event(
            event_type,
            version,
            causality_id,
            kernel_policy_revision,
            payload,
        )
    }

    pub fn register_durable_schema(&self, schema: &DurableSchema) -> Result<(), KernelError> {
        self.host.register_durable_schema(schema)
    }

    pub fn migrate_durable_schema(
        &self,
        schema: &DurableSchema,
        migrations: &[SchemaMigration],
    ) -> Result<(), KernelError> {
        self.host.migrate_durable_schema(schema, migrations)
    }

    pub fn read_durable(
        &self,
        namespace: &ResourceNamespace,
        key: &str,
    ) -> Result<Option<Vec<u8>>, KernelError> {
        self.host.read_durable(namespace, key)
    }

    pub fn transact_durable(
        &self,
        namespace: &ResourceNamespace,
        operations: &[TransactionOp],
    ) -> Result<(), KernelError> {
        self.host.transact_durable(namespace, operations)
    }

    pub fn transact_durable_many(
        &self,
        transactions: &[NamespaceTransaction],
    ) -> Result<(), KernelError> {
        self.host.transact_durable_many(transactions)
    }
}

/// Marker implemented by a typed SDK contract supplied by another plugin.
pub trait SdkContract {
    type Interface: ComponentInterface;
}

/// Kernel-mediated client for one typed imported interface.
pub struct SdkClient<'host, 'runtime, I: ComponentInterface> {
    host: &'host PluginHost<'runtime>,
    component: ComponentId,
    interface: PhantomData<fn() -> I>,
}

impl<'host, 'runtime, I: ComponentInterface> Clone for SdkClient<'host, 'runtime, I> {
    fn clone(&self) -> Self {
        Self {
            host: self.host,
            component: self.component.clone(),
            interface: PhantomData,
        }
    }
}

impl<'host, 'runtime, I: ComponentInterface> SdkClient<'host, 'runtime, I> {
    pub fn new(host: &'host PluginHost<'runtime>, component: ComponentId) -> Self {
        Self {
            host,
            component,
            interface: PhantomData,
        }
    }

    pub fn component(&self) -> &ComponentId {
        &self.component
    }

    pub fn invoke(&self, request: &I::Request) -> Result<I::Response, ComponentInvocationError> {
        self.host.invoke_import::<I>(&self.component, request)
    }
}

/// Consumer-side handle for a provider-owned SDK object.
///
/// The handle contains stable identity plus a scoped typed client. Provider
/// state remains owned by the providing plugin.
pub struct SdkObject<'host, 'runtime, I: ComponentInterface, Id = String> {
    id: Id,
    client: SdkClient<'host, 'runtime, I>,
}

impl<'host, 'runtime, I, Id> Clone for SdkObject<'host, 'runtime, I, Id>
where
    I: ComponentInterface,
    Id: Clone,
{
    fn clone(&self) -> Self {
        Self {
            id: self.id.clone(),
            client: self.client.clone(),
        }
    }
}

impl<'host, 'runtime, I: ComponentInterface, Id> SdkObject<'host, 'runtime, I, Id> {
    pub fn new(id: Id, client: SdkClient<'host, 'runtime, I>) -> Self {
        Self { id, client }
    }

    pub fn id(&self) -> &Id {
        &self.id
    }

    pub fn client(&self) -> &SdkClient<'host, 'runtime, I> {
        &self.client
    }

    pub fn into_id(self) -> Id {
        self.id
    }
}
