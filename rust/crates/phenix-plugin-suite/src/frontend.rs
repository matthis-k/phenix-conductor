use crate::{execution_service, ExecutionCommand, ExecutionResponse, ExecutionState};
use phenix_kernel::{
    Authority, PluginExecution, PluginHost, PluginId, PluginInstance, PluginManifest,
    ServiceContribution, ServiceId,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const FRONTEND_SERVICE: &str = "phenix.frontend-services@1";
const FRONTEND_PLUGIN: &str = "phenix.frontend-services";

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct FrontendProviderDescriptor {
    pub id: String,
    #[serde(default)]
    pub capabilities: BTreeSet<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct LiveFrontendProvider {
    pub connection_id: String,
    pub descriptor: FrontendProviderDescriptor,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct FrontendServiceRequest {
    pub correlation_id: u64,
    pub connection_id: String,
    pub provider: String,
    pub method: String,
    pub params: serde_json::Value,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct FrontendServiceResult {
    pub correlation_id: u64,
    pub result: serde_json::Value,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "operation", rename_all = "snake_case")]
pub enum FrontendCommand {
    SetProviders {
        connection_id: String,
        providers: Vec<FrontendProviderDescriptor>,
    },
    Disconnect {
        connection_id: String,
    },
    Catalog,
    BindRoot {
        execution_id: String,
        connection_id: String,
    },
    ReleaseRoot {
        execution_id: String,
    },
    BeginExecutionCall {
        execution_id: String,
        provider: String,
        method: String,
        params: serde_json::Value,
    },
    BeginDirectCall {
        connection_id: String,
        provider: String,
        method: String,
        params: serde_json::Value,
    },
    CompleteCall {
        connection_id: String,
        correlation_id: u64,
        result: serde_json::Value,
    },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "response", rename_all = "snake_case")]
pub enum FrontendResponse {
    Providers {
        providers: Vec<LiveFrontendProvider>,
    },
    Request {
        request: FrontendServiceRequest,
    },
    Result {
        result: FrontendServiceResult,
    },
    Updated,
}

#[must_use]
pub fn frontend_manifest(maximum_authority: Authority) -> PluginManifest {
    PluginManifest {
        id: PluginId::parse(FRONTEND_PLUGIN).expect("static plugin id is valid"),
        version: 1,
        execution: PluginExecution::Embedded,
        dependencies: Vec::new(),
        services: vec![ServiceContribution {
            service: frontend_service(),
            priority: 100,
            required_authority: Authority::default(),
        }],
        resource_namespaces: Vec::new(),
        maximum_authority,
    }
}

#[must_use]
pub fn frontend_factory() -> Box<dyn PluginInstance> {
    Box::new(FrontendPlugin::default())
}

#[must_use]
pub fn frontend_service() -> ServiceId {
    ServiceId::parse(FRONTEND_SERVICE).expect("static service id is valid")
}

#[derive(Clone, Debug)]
struct PendingCall {
    connection_id: String,
}

#[derive(Default)]
struct FrontendPlugin {
    providers: BTreeMap<String, BTreeMap<String, FrontendProviderDescriptor>>,
    root_routes: BTreeMap<String, String>,
    pending: BTreeMap<u64, PendingCall>,
    next_correlation_id: u64,
}

impl PluginInstance for FrontendPlugin {
    fn start(&mut self, _host: &PluginHost<'_>) -> Result<(), String> {
        self.next_correlation_id = 1;
        Ok(())
    }

    fn invoke(
        &mut self,
        service: &ServiceId,
        input: &[u8],
        host: &PluginHost<'_>,
    ) -> Result<Vec<u8>, String> {
        if service != &frontend_service() {
            return Err(format!("unsupported frontend service: {service}"));
        }
        let command: FrontendCommand =
            serde_json::from_slice(input).map_err(|error| error.to_string())?;
        let response = match command {
            FrontendCommand::SetProviders {
                connection_id,
                providers,
            } => {
                validate_id("frontend connection id", &connection_id)?;
                let mut indexed = BTreeMap::new();
                for provider in providers {
                    validate_provider(&provider)?;
                    if indexed.insert(provider.id.clone(), provider).is_some() {
                        return Err("duplicate frontend provider id in advertisement".into());
                    }
                }
                self.providers.insert(connection_id, indexed);
                FrontendResponse::Updated
            }
            FrontendCommand::Disconnect { connection_id } => {
                self.providers.remove(&connection_id);
                self.root_routes.retain(|_, owner| owner != &connection_id);
                self.pending
                    .retain(|_, call| call.connection_id != connection_id);
                FrontendResponse::Updated
            }
            FrontendCommand::Catalog => FrontendResponse::Providers {
                providers: self.catalog(),
            },
            FrontendCommand::BindRoot {
                execution_id,
                connection_id,
            } => {
                validate_id("execution id", &execution_id)?;
                if !self.providers.contains_key(&connection_id) {
                    return Err(format!("unknown frontend connection: {connection_id}"));
                }
                let execution = execution_lookup(host, &execution_id)?
                    .ok_or_else(|| format!("unknown execution: {execution_id}"))?;
                if execution.parent_execution.is_some() {
                    return Err("only root executions may be bound to a frontend connection".into());
                }
                if !matches!(execution.state, ExecutionState::Active) {
                    return Err("only active root executions may be bound".into());
                }
                if self
                    .root_routes
                    .insert(execution_id.clone(), connection_id)
                    .is_some()
                {
                    return Err(format!(
                        "root execution already has a frontend route: {execution_id}"
                    ));
                }
                FrontendResponse::Updated
            }
            FrontendCommand::ReleaseRoot { execution_id } => {
                self.root_routes.remove(&execution_id);
                FrontendResponse::Updated
            }
            FrontendCommand::BeginExecutionCall {
                execution_id,
                provider,
                method,
                params,
            } => {
                let root = execution_root(host, &execution_id)?;
                let connection_id = self.root_routes.get(&root).cloned().ok_or_else(|| {
                    format!("execution has no live frontend root route: {execution_id}")
                })?;
                self.begin_call(connection_id, provider, method, params)?
            }
            FrontendCommand::BeginDirectCall {
                connection_id,
                provider,
                method,
                params,
            } => self.begin_call(connection_id, provider, method, params)?,
            FrontendCommand::CompleteCall {
                connection_id,
                correlation_id,
                result,
            } => {
                let pending = self
                    .pending
                    .remove(&correlation_id)
                    .ok_or_else(|| format!("unknown frontend correlation id: {correlation_id}"))?;
                if pending.connection_id != connection_id {
                    self.pending.insert(correlation_id, pending);
                    return Err("frontend response came from the wrong connection".into());
                }
                FrontendResponse::Result {
                    result: FrontendServiceResult {
                        correlation_id,
                        result,
                    },
                }
            }
        };
        serde_json::to_vec(&response).map_err(|error| error.to_string())
    }
}

impl FrontendPlugin {
    fn catalog(&self) -> Vec<LiveFrontendProvider> {
        let mut result = Vec::new();
        for (connection_id, providers) in &self.providers {
            for descriptor in providers.values() {
                result.push(LiveFrontendProvider {
                    connection_id: connection_id.clone(),
                    descriptor: descriptor.clone(),
                });
            }
        }
        result
    }

    fn begin_call(
        &mut self,
        connection_id: String,
        provider: String,
        method: String,
        params: serde_json::Value,
    ) -> Result<FrontendResponse, String> {
        validate_id("frontend connection id", &connection_id)?;
        validate_id("frontend provider id", &provider)?;
        validate_id("frontend method", &method)?;
        if !self
            .providers
            .get(&connection_id)
            .is_some_and(|providers| providers.contains_key(&provider))
        {
            return Err(format!(
                "frontend connection {connection_id} does not advertise provider {provider}"
            ));
        }
        let correlation_id = self.next_correlation_id;
        self.next_correlation_id = self
            .next_correlation_id
            .checked_add(1)
            .ok_or_else(|| "frontend correlation id exhausted".to_owned())?;
        self.pending.insert(
            correlation_id,
            PendingCall {
                connection_id: connection_id.clone(),
            },
        );
        Ok(FrontendResponse::Request {
            request: FrontendServiceRequest {
                correlation_id,
                connection_id,
                provider,
                method,
                params,
            },
        })
    }
}

fn execution_lookup(
    host: &PluginHost<'_>,
    execution_id: &str,
) -> Result<Option<crate::ExecutionRecord>, String> {
    let output = host
        .invoke_service(
            &execution_service(),
            &serde_json::to_vec(&ExecutionCommand::GetExecution {
                id: execution_id.to_owned(),
            })
            .map_err(|error| error.to_string())?,
            host.authority(),
            None,
        )
        .map_err(|error| error.to_string())?;
    match serde_json::from_slice::<ExecutionResponse>(&output).map_err(|error| error.to_string())? {
        ExecutionResponse::ExecutionLookup { execution } => Ok(execution),
        other => Err(format!("unexpected execution lookup response: {other:?}")),
    }
}

fn execution_root(host: &PluginHost<'_>, execution_id: &str) -> Result<String, String> {
    let mut current = execution_id.to_owned();
    let mut visited = BTreeSet::new();
    loop {
        if !visited.insert(current.clone()) {
            return Err("execution parent cycle while resolving frontend route".into());
        }
        let execution = execution_lookup(host, &current)?
            .ok_or_else(|| format!("unknown execution: {current}"))?;
        match execution.parent_execution {
            Some(parent) => current = parent,
            None => return Ok(current),
        }
    }
}

fn validate_provider(provider: &FrontendProviderDescriptor) -> Result<(), String> {
    validate_id("frontend provider id", &provider.id)?;
    if provider
        .capabilities
        .iter()
        .any(|capability| capability.trim().is_empty())
    {
        return Err("frontend provider capabilities must not be empty strings".into());
    }
    Ok(())
}

fn validate_id(label: &str, value: &str) -> Result<(), String> {
    if value.trim().is_empty() {
        Err(format!("{label} must not be empty"))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{execution_factory, execution_manifest, ExecutionAuthority};
    use phenix_kernel::{Kernel, KernelConfig};

    fn kernel() -> Kernel {
        let execution_manifest = execution_manifest(Authority::default());
        let execution_id = execution_manifest.id.clone();
        let frontend_manifest = frontend_manifest(execution_manifest.maximum_authority.clone());
        let frontend_id = frontend_manifest.id.clone();
        let mut kernel =
            Kernel::new(KernelConfig::new([execution_manifest, frontend_manifest]).unwrap());
        kernel
            .register_embedded_factory(execution_id, execution_factory)
            .unwrap();
        kernel
            .register_embedded_factory(frontend_id, frontend_factory)
            .unwrap();
        kernel.activate_all().unwrap();
        kernel
    }

    fn invoke(kernel: &mut Kernel, command: FrontendCommand) -> Result<FrontendResponse, String> {
        let output = kernel
            .invoke(
                &frontend_service(),
                &serde_json::to_vec(&command).unwrap(),
                &Authority::default(),
                None,
            )
            .map_err(|error| error.to_string())?;
        serde_json::from_slice(&output).map_err(|error| error.to_string())
    }

    fn execution(kernel: &mut Kernel, id: &str, parent: Option<&str>) {
        let command = match parent {
            None => ExecutionCommand::CreateExecution {
                id: id.into(),
                requested_authority: ExecutionAuthority::new(Vec::<String>::new()),
            },
            Some(parent) => ExecutionCommand::DelegateExecution {
                parent_execution: parent.into(),
                id: id.into(),
                requested_authority: ExecutionAuthority::new(Vec::<String>::new()),
            },
        };
        kernel
            .invoke(
                &execution_service(),
                &serde_json::to_vec(&command).unwrap(),
                &Authority::default(),
                None,
            )
            .unwrap();
    }

    #[test]
    fn descendant_calls_route_to_root_owner_and_wrong_connection_response_is_rejected() {
        let mut kernel = kernel();
        execution(&mut kernel, "root", None);
        execution(&mut kernel, "child", Some("root"));
        invoke(
            &mut kernel,
            FrontendCommand::SetProviders {
                connection_id: "frontend-a".into(),
                providers: vec![FrontendProviderDescriptor {
                    id: "web".into(),
                    capabilities: BTreeSet::from(["search".into()]),
                }],
            },
        )
        .unwrap();
        invoke(
            &mut kernel,
            FrontendCommand::BindRoot {
                execution_id: "root".into(),
                connection_id: "frontend-a".into(),
            },
        )
        .unwrap();
        let response = invoke(
            &mut kernel,
            FrontendCommand::BeginExecutionCall {
                execution_id: "child".into(),
                provider: "web".into(),
                method: "search".into(),
                params: serde_json::json!({"q":"nix"}),
            },
        )
        .unwrap();
        let correlation_id = match response {
            FrontendResponse::Request { request } => {
                assert_eq!(request.connection_id, "frontend-a");
                request.correlation_id
            }
            other => panic!("unexpected response: {other:?}"),
        };
        assert!(invoke(
            &mut kernel,
            FrontendCommand::CompleteCall {
                connection_id: "frontend-b".into(),
                correlation_id,
                result: serde_json::json!({}),
            }
        )
        .unwrap_err()
        .contains("wrong connection"));
        assert!(matches!(
            invoke(
                &mut kernel,
                FrontendCommand::CompleteCall {
                    connection_id: "frontend-a".into(),
                    correlation_id,
                    result: serde_json::json!({"ok":true}),
                }
            )
            .unwrap(),
            FrontendResponse::Result { .. }
        ));
    }

    #[test]
    fn disconnect_removes_provider_catalog_routes_and_pending_calls_without_durable_restore() {
        let mut kernel = kernel();
        execution(&mut kernel, "root", None);
        invoke(
            &mut kernel,
            FrontendCommand::SetProviders {
                connection_id: "frontend-a".into(),
                providers: vec![FrontendProviderDescriptor {
                    id: "web".into(),
                    capabilities: BTreeSet::new(),
                }],
            },
        )
        .unwrap();
        invoke(
            &mut kernel,
            FrontendCommand::BindRoot {
                execution_id: "root".into(),
                connection_id: "frontend-a".into(),
            },
        )
        .unwrap();
        invoke(
            &mut kernel,
            FrontendCommand::Disconnect {
                connection_id: "frontend-a".into(),
            },
        )
        .unwrap();
        assert_eq!(
            invoke(&mut kernel, FrontendCommand::Catalog).unwrap(),
            FrontendResponse::Providers {
                providers: Vec::new()
            }
        );
        assert!(invoke(
            &mut kernel,
            FrontendCommand::BeginExecutionCall {
                execution_id: "root".into(),
                provider: "web".into(),
                method: "search".into(),
                params: serde_json::json!({}),
            }
        )
        .unwrap_err()
        .contains("no live frontend root route"));
    }
}
