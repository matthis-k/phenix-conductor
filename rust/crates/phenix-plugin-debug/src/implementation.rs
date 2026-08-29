use crate::debug_component_id;
use phenix_core::{
    Authority, ComponentInterface, ComponentInvocationError, PluginContext, PluginExecution,
    PluginHost, PluginId, PluginInstance, PluginManifest, SdkClient, ServiceContribution,
    ServiceId,
};
use phenix_plugin_context::{ContextCommand, ContextInterface};
use phenix_plugin_frontend::{FrontendCommand, FrontendInterface};
use phenix_plugin_jobs::{JobCommand, JobInterface};
use phenix_plugin_models::{ModelCommand, ModelRoutingInterface};
use phenix_plugin_planning::{PlanningCommand, PlanningInterface};
use phenix_plugin_sessions::{SessionCommand, SessionInterface};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const DEBUG_SERVICE: &str = "phenix.debug@1";
const DEBUG_PLUGIN: &str = "phenix.debug";

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "operation", rename_all = "snake_case")]
pub enum DebugCommand {
    Snapshot,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DiagnosticEntry {
    pub available: bool,
    pub value: serde_json::Value,
    pub error: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DiagnosticSnapshot {
    pub services: BTreeMap<String, DiagnosticEntry>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "response", rename_all = "snake_case")]
pub enum DebugResponse {
    Snapshot { snapshot: DiagnosticSnapshot },
}

#[must_use]
pub fn debug_manifest(maximum_authority: Authority) -> PluginManifest {
    PluginManifest {
        id: PluginId::parse(DEBUG_PLUGIN).expect("static plugin id is valid"),
        version: 1,
        execution: PluginExecution::Embedded,
        dependencies: Vec::new(),
        services: vec![ServiceContribution {
            role: phenix_core::ServiceRole::Terminal,
            service: debug_service(),
            priority: 100,
            required_authority: Authority::default(),
        }],
        resource_namespaces: Vec::new(),
        maximum_authority,
    }
}

#[must_use]
pub fn debug_factory() -> Box<dyn PluginInstance> {
    Box::new(DebugPlugin)
}

#[must_use]
pub fn debug_service() -> ServiceId {
    ServiceId::parse(DEBUG_SERVICE).expect("static service id is valid")
}

struct DebugSdk<'host, 'runtime> {
    sessions: SdkClient<'host, 'runtime, SessionInterface>,
    context: SdkClient<'host, 'runtime, ContextInterface>,
    planning: SdkClient<'host, 'runtime, PlanningInterface>,
    jobs: SdkClient<'host, 'runtime, JobInterface>,
    models: SdkClient<'host, 'runtime, ModelRoutingInterface>,
    frontends: SdkClient<'host, 'runtime, FrontendInterface>,
}

type DebugContext<'host, 'runtime> = PluginContext<'host, 'runtime, DebugSdk<'host, 'runtime>>;

fn context<'host, 'runtime>(host: &'host PluginHost<'runtime>) -> DebugContext<'host, 'runtime> {
    let component = debug_component_id();
    PluginContext::new(
        host,
        DebugSdk {
            sessions: SdkClient::new(host, component.clone()),
            context: SdkClient::new(host, component.clone()),
            planning: SdkClient::new(host, component.clone()),
            jobs: SdkClient::new(host, component.clone()),
            models: SdkClient::new(host, component.clone()),
            frontends: SdkClient::new(host, component),
        },
        (),
        (),
    )
}

struct DebugPlugin;

impl PluginInstance for DebugPlugin {
    fn start(&mut self, _host: &PluginHost<'_>) -> Result<(), String> {
        Ok(())
    }

    fn invoke(
        &mut self,
        service: &ServiceId,
        input: &[u8],
        host: &PluginHost<'_>,
    ) -> Result<Vec<u8>, String> {
        if service != &debug_service() {
            return Err(format!("unsupported debug service: {service}"));
        }
        let command = serde_json::from_slice(input).map_err(|error| error.to_string())?;
        let response = handle(&context(host), command);
        serde_json::to_vec(&response).map_err(|error| error.to_string())
    }
}

fn handle(context: &DebugContext<'_, '_>, command: DebugCommand) -> DebugResponse {
    match command {
        DebugCommand::Snapshot => DebugResponse::Snapshot {
            snapshot: snapshot(context),
        },
    }
}

fn snapshot(context: &DebugContext<'_, '_>) -> DiagnosticSnapshot {
    let mut services = BTreeMap::new();
    probe(
        &context.sdk.sessions,
        &mut services,
        "sessions",
        &SessionCommand::List,
    );
    probe(
        &context.sdk.context,
        &mut services,
        "context",
        &ContextCommand::List,
    );
    probe(
        &context.sdk.planning,
        &mut services,
        "planning_history",
        &PlanningCommand::SearchHistory {
            objective_id: None,
            query: String::new(),
        },
    );
    probe(&context.sdk.jobs, &mut services, "jobs", &JobCommand::List);
    probe(
        &context.sdk.models,
        &mut services,
        "models",
        &ModelCommand::ListProfiles,
    );
    probe(
        &context.sdk.frontends,
        &mut services,
        "frontends",
        &FrontendCommand::Catalog,
    );
    DiagnosticSnapshot { services }
}

fn probe<I>(
    client: &SdkClient<'_, '_, I>,
    entries: &mut BTreeMap<String, DiagnosticEntry>,
    name: &str,
    command: &I::Request,
) where
    I: ComponentInterface,
    I::Response: Serialize,
{
    let entry = match client.invoke(command) {
        Ok(response) => match serde_json::to_value(response) {
            Ok(value) => DiagnosticEntry {
                available: true,
                value,
                error: None,
            },
            Err(error) => DiagnosticEntry {
                available: false,
                value: serde_json::Value::Null,
                error: Some(format!("invalid diagnostic service response: {error}")),
            },
        },
        Err(error @ ComponentInvocationError::UnboundImport { .. }) => DiagnosticEntry {
            available: false,
            value: serde_json::Value::Null,
            error: Some(error.to_string()),
        },
        Err(error) => DiagnosticEntry {
            available: false,
            value: serde_json::Value::Null,
            error: Some(error.to_string()),
        },
    };
    entries.insert(name.into(), entry);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::debug_component_manifest;
    use phenix_core::{Kernel, KernelConfig, ResolvedHarness, ResolvedHarnessActivation};
    use phenix_plugin_sessions::{session_component_manifest, session_factory, session_manifest};

    #[test]
    fn diagnostic_service_uses_resolved_optional_imports_without_kernel_fallbacks() {
        let session_manifest = session_manifest();
        let authority = session_manifest.maximum_authority.clone();
        let session_id = session_manifest.id.clone();
        let debug_manifest = debug_manifest(authority.clone());
        let debug_id = debug_manifest.id.clone();
        let manifests = vec![session_manifest, debug_manifest];
        let resolved = ResolvedHarness::resolve(
            manifests.clone(),
            [
                session_component_manifest(),
                debug_component_manifest(authority.clone()),
            ],
            [],
            &authority,
        )
        .unwrap();
        let mut kernel = Kernel::new(KernelConfig::new(manifests).unwrap());
        kernel
            .register_embedded_factory(session_id, session_factory)
            .unwrap();
        kernel
            .register_embedded_factory(debug_id, debug_factory)
            .unwrap();
        kernel.activate_resolved_harness(&resolved).unwrap();
        kernel.activate_all().unwrap();

        let output = kernel
            .invoke(
                &debug_service(),
                &serde_json::to_vec(&DebugCommand::Snapshot).unwrap(),
                &authority,
                None,
            )
            .unwrap();
        let DebugResponse::Snapshot { snapshot } = serde_json::from_slice(&output).unwrap();
        assert!(snapshot.services["sessions"].available);
        assert!(!snapshot.services["context"].available);
        assert!(!snapshot.services["models"].available);
    }
}
