use super::{
    dispatch::{
        invoke_component_service_with, invoke_resolved_chain_with, invoke_service_with,
        ServiceDispatchGuards,
    },
    *,
};

impl<'a> PluginHost<'a> {
    pub fn graph_generation(&self) -> Option<&GraphGenerationId> {
        self.graph_generation
    }

    pub fn plugin(&self) -> &PluginId {
        self.plugin
    }

    pub fn authority(&self) -> &Authority {
        self.authority
    }

    pub fn invoke_import<I: ComponentInterface>(
        &self,
        component: &ComponentId,
        request: &crate::PhenixValue,
    ) -> Result<crate::PhenixValue, ComponentInvocationError> {
        let resolved = self
            .component_graph
            .component(component)
            .ok_or_else(|| crate::ComponentGraphError::UnknownComponent(component.clone()))?;
        if &resolved.owning_plugin != self.plugin {
            return Err(KernelError::HostOperationDenied {
                plugin: self.plugin.clone(),
                operation: format!("component import owned by {}", resolved.owning_plugin),
            }
            .into());
        }
        let interface = I::interface_id();
        let handle = self
            .component_graph
            .import_handle(component, &interface)?
            .ok_or_else(|| ComponentInvocationError::UnboundImport {
                component: component.clone(),
                interface: interface.clone(),
            })?;
        let service = ServiceId::parse(interface.as_str().to_owned()).map_err(|message| {
            ComponentInvocationError::InvalidInterface {
                interface: interface.clone(),
                message: message.into(),
            }
        })?;
        let input = serde_json::to_vec(request)
            .map_err(|error| ComponentInvocationError::Encode(error.to_string()))?;
        let delegated_authority = self.authority.attenuate(handle.effective_authority());
        let output = invoke_component_service_with(
            InvocationContext {
                graph_generation: self.graph_generation,
                component_graph: self.component_graph,
                config: self.config,
                states: self.states,
                instances: self.instances,
                events: self.events,
                tasks: self.tasks,
                persistence: self.persistence,
                provenance: self.provenance,
            },
            &service,
            handle.exporter(),
            &input,
            &delegated_authority,
            handle.owning_plugin(),
            ServiceDispatchGuards {
                call_stack: &self.call_stack,
                active_services: &self.active_services,
                terminal_component: Some(handle.exporter()),
            },
        )?;
        serde_json::from_slice(&output)
            .map_err(|error| ComponentInvocationError::Decode(error.to_string()))
    }

    #[doc(hidden)]
    pub fn invoke_service_abi(
        &self,
        service: &ServiceId,
        input: &[u8],
        requested_authority: &Authority,
        binding: Option<&PluginId>,
    ) -> Result<Vec<u8>, KernelError> {
        let delegated_authority = self.authority.attenuate(requested_authority);
        invoke_service_with(
            InvocationContext {
                graph_generation: self.graph_generation,
                component_graph: self.component_graph,
                config: self.config,
                states: self.states,
                instances: self.instances,
                events: self.events,
                tasks: self.tasks,
                persistence: self.persistence,
                provenance: self.provenance,
            },
            service,
            input,
            &delegated_authority,
            binding,
            &self.call_stack,
            &self.active_services,
        )
    }

    pub fn continue_service(
        &self,
        input: &[u8],
        requested_authority: &Authority,
    ) -> Result<Vec<u8>, KernelError> {
        let continuation = self
            .continuation
            .as_ref()
            .ok_or(KernelError::ContinuationUnavailable)?;
        let service = continuation.chain.service.clone();
        if continuation.used.swap(true, Ordering::AcqRel) {
            return Err(KernelError::ContinuationAlreadyUsed(service));
        }
        let delegated_authority = self.authority.attenuate(requested_authority);
        invoke_resolved_chain_with(
            InvocationContext {
                graph_generation: self.graph_generation,
                component_graph: self.component_graph,
                config: self.config,
                states: self.states,
                instances: self.instances,
                events: self.events,
                tasks: self.tasks,
                persistence: self.persistence,
                provenance: self.provenance,
            },
            &continuation.chain,
            continuation.next_position,
            input,
            &delegated_authority,
            ServiceDispatchGuards {
                call_stack: &self.call_stack,
                active_services: &self.active_services,
                terminal_component: continuation.terminal_component.as_ref(),
            },
            &continuation.trace,
        )
    }

    pub(crate) fn continuation_binding(&self) -> Result<ContinuationBinding, KernelError> {
        let continuation = self
            .continuation
            .as_ref()
            .ok_or(KernelError::ContinuationUnavailable)?;
        Ok(ContinuationBinding {
            graph_generation: self.graph_generation.cloned(),
            policy_identity: continuation.chain.policy_identity,
            service: continuation.chain.service.clone(),
            authority: self.authority.clone(),
            next_position: continuation.next_position,
        })
    }

    pub fn spawn_task<T, F>(&self, requested_authority: &Authority, worker: F) -> TaskHandle<T>
    where
        T: Send + 'static,
        F: FnOnce(crate::CancellationToken) -> T + Send + 'static,
    {
        self.tasks.spawn(
            self.graph_generation
                .expect("plugin host task spawn requires a resolved graph generation"),
            self.authority,
            requested_authority,
            worker,
        )
    }

    pub fn dispatch_event(
        &self,
        event_type: EventTypeId,
        version: u32,
        causality_id: u64,
        kernel_policy_revision: u64,
        payload: Vec<u8>,
    ) -> Result<EventDispatchReport, EventError> {
        let event = EventEnvelope {
            event_type,
            version,
            emitter: self.plugin.clone(),
            causality_id,
            kernel_policy_revision,
            payload,
        };
        self.events.dispatch(&event, self.authority)
    }

    pub fn register_durable_schema(&self, schema: &DurableSchema) -> Result<(), KernelError> {
        self.require_persistence_operation(PERSISTENCE_SCHEMA, &schema.namespace)?;
        self.persistence
            .lock()
            .expect("kernel persistence mutex poisoned")
            .register_schema(self.plugin, schema)
            .map_err(|error| self.persistence_error(error.to_string()))
    }

    pub fn migrate_durable_schema(
        &self,
        schema: &DurableSchema,
        migrations: &[SchemaMigration],
    ) -> Result<(), KernelError> {
        self.require_persistence_operation(PERSISTENCE_SCHEMA, &schema.namespace)?;
        self.require_capability(PERSISTENCE_WRITE)?;
        self.persistence
            .lock()
            .expect("kernel persistence mutex poisoned")
            .migrate_schema(self.plugin, schema, migrations)
            .map_err(|error| self.persistence_error(error.to_string()))
    }

    pub fn read_durable(
        &self,
        namespace: &ResourceNamespace,
        key: &str,
    ) -> Result<Option<Vec<u8>>, KernelError> {
        self.require_persistence_operation(PERSISTENCE_READ, namespace)?;
        self.persistence
            .lock()
            .expect("kernel persistence mutex poisoned")
            .read(self.plugin, namespace, key)
            .map_err(|error| self.persistence_error(error.to_string()))
    }

    pub fn transact_durable(
        &self,
        namespace: &ResourceNamespace,
        operations: &[TransactionOp],
    ) -> Result<(), KernelError> {
        self.require_persistence_operation(PERSISTENCE_WRITE, namespace)?;
        self.persistence
            .lock()
            .expect("kernel persistence mutex poisoned")
            .transact(self.plugin, namespace, operations)
            .map_err(|error| self.persistence_error(error.to_string()))
    }

    pub fn transact_durable_many(
        &self,
        transactions: &[NamespaceTransaction],
    ) -> Result<(), KernelError> {
        self.require_capability(PERSISTENCE_WRITE)?;
        if !transactions
            .iter()
            .any(|transaction| &transaction.owner == self.plugin)
        {
            return Err(KernelError::HostOperationDenied {
                plugin: self.plugin.clone(),
                operation: "multi-namespace transaction requires a caller-owned participant".into(),
            });
        }
        for transaction in transactions {
            if self.config.resource_owner(&transaction.namespace) != Some(&transaction.owner) {
                return Err(KernelError::HostOperationDenied {
                    plugin: self.plugin.clone(),
                    operation: format!(
                        "{PERSISTENCE_WRITE}:{}:{}",
                        transaction.owner, transaction.namespace
                    ),
                });
            }
            if &transaction.owner == self.plugin {
                continue;
            }
            let write = CapabilityId::parse(PERSISTENCE_WRITE)
                .expect("kernel persistence write capability is valid");
            let authorized_import = self
                .component_graph
                .components()
                .filter(|component| &component.owning_plugin == self.plugin)
                .flat_map(|component| component.imports.iter())
                .filter_map(|import| import.binding.as_ref())
                .any(|binding| {
                    binding.owning_plugin() == &transaction.owner
                        && binding.effective_authority().permits(&write)
                });
            if !authorized_import {
                return Err(KernelError::HostOperationDenied {
                    plugin: self.plugin.clone(),
                    operation: format!(
                        "{PERSISTENCE_WRITE}:{}:{} without authorized typed import",
                        transaction.owner, transaction.namespace
                    ),
                });
            }
        }
        self.persistence
            .lock()
            .expect("kernel persistence mutex poisoned")
            .transact_many(transactions)
            .map_err(|error| self.persistence_error(error.to_string()))
    }

    fn require_persistence_operation(
        &self,
        capability: &str,
        namespace: &ResourceNamespace,
    ) -> Result<(), KernelError> {
        self.require_capability(capability)?;
        if self.config.resource_owner(namespace) == Some(self.plugin) {
            return Ok(());
        }
        Err(KernelError::HostOperationDenied {
            plugin: self.plugin.clone(),
            operation: format!("{capability}:{}", namespace.as_str()),
        })
    }

    fn require_capability(&self, capability: &str) -> Result<(), KernelError> {
        let capability = CapabilityId::parse(capability).expect("kernel capability is valid");
        if self.authority.permits(&capability) {
            return Ok(());
        }
        Err(KernelError::HostOperationDenied {
            plugin: self.plugin.clone(),
            operation: capability.as_str().to_owned(),
        })
    }

    fn persistence_error(&self, message: String) -> KernelError {
        KernelError::Persistence {
            plugin: self.plugin.clone(),
            message,
        }
    }
}
