use crate::debug_component_id;
use phenix_core::{
    Authority, ComponentInterface, ComponentInvocationError, PluginExecution, PluginHost, PluginId,
    PluginInstance, PluginManifest, ServiceContribution, ServiceId,
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
        let command: DebugCommand =
            serde_json::from_slice(input).map_err(|error| error.to_string())?;
        let response = match command {
            DebugCommand::Snapshot => DebugResponse::Snapshot {
                snapshot: snapshot(host),
            },
        };
        serde_json::to_vec(&response).map_err(|error| error.to_string())
    }
}

fn snapshot(host: &PluginHost<'_>) -> DiagnosticSnapshot {
    let mut services = BTreeMap::new();
    probe::<SessionInterface>(host, &mut services, "sessions", &SessionCommand::List);
    probe::<ContextInterface>(host, &mut services, "context", &ContextCommand::List);
    probe::<PlanningInterface>(
        host,
        &mut services,
        "planning_history",
        &PlanningCommand::SearchHistory {
            objective_id: None,
            query: String::new(),
        },
    );
    probe::<JobInterface>(host, &mut services, "jobs", &JobCommand::List);
    probe::<ModelRoutingInterface>(host, &mut services, "models", &ModelCommand::ListProfiles);
    probe::<FrontendInterface>(host, &mut services, "frontends", &FrontendCommand::Catalog);
    DiagnosticSnapshot { services }
}

fn probe<I>(
    host: &PluginHost<'_>,
    entries: &mut BTreeMap<String, DiagnosticEntry>,
    name: &str,
    command: &I::Request,
) where
    I: ComponentInterface,
    I::Response: Serialize,
{
    let entry = match host.invoke_import::<I>(&debug_component_id(), command) {
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
