use crate::{
    context_service, frontend_service, job_service, model_routing_service, planning_service,
    session_service, ContextCommand, FrontendCommand, JobCommand, ModelCommand, PlanningCommand,
    SessionCommand,
};
use phenix_kernel::{
    Authority, PluginExecution, PluginHost, PluginId, PluginInstance, PluginManifest,
    ServiceContribution, ServiceId,
};
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
        let command: DebugCommand = serde_json::from_slice(input).map_err(|error| error.to_string())?;
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
    probe(host, &mut services, "sessions", &session_service(), &SessionCommand::List);
    probe(host, &mut services, "context", &context_service(), &ContextCommand::List);
    probe(
        host,
        &mut services,
        "planning_history",
        &planning_service(),
        &PlanningCommand::SearchHistory {
            objective_id: None,
            query: String::new(),
        },
    );
    probe(host, &mut services, "jobs", &job_service(), &JobCommand::List);
    probe(
        host,
        &mut services,
        "models",
        &model_routing_service(),
        &ModelCommand::ListProfiles,
    );
    probe(
        host,
        &mut services,
        "frontends",
        &frontend_service(),
        &FrontendCommand::Catalog,
    );
    DiagnosticSnapshot { services }
}

fn probe<T: Serialize>(
    host: &PluginHost<'_>,
    entries: &mut BTreeMap<String, DiagnosticEntry>,
    name: &str,
    service: &ServiceId,
    command: &T,
) {
    let input = match serde_json::to_vec(command) {
        Ok(input) => input,
        Err(error) => {
            entries.insert(
                name.into(),
                DiagnosticEntry {
                    available: false,
                    value: serde_json::Value::Null,
                    error: Some(error.to_string()),
                },
            );
            return;
        }
    };
    let entry = match host.invoke_service(service, &input, host.authority(), None) {
        Ok(output) => match serde_json::from_slice(&output) {
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
    use crate::{session_factory, session_manifest};
    use phenix_kernel::{Kernel, KernelConfig};

    #[test]
    fn diagnostic_service_reports_available_and_omitted_plugins_without_kernel_fallbacks() {
        let session_manifest = session_manifest();
        let session_id = session_manifest.id.clone();
        let debug_manifest = debug_manifest(session_manifest.maximum_authority.clone());
        let debug_id = debug_manifest.id.clone();
        let mut kernel = Kernel::new(KernelConfig::new([session_manifest, debug_manifest]).unwrap());
        kernel.register_embedded_factory(session_id, session_factory).unwrap();
        kernel.register_embedded_factory(debug_id, debug_factory).unwrap();
        kernel.activate_all().unwrap();
        let output = kernel.invoke(
            &debug_service(),
            &serde_json::to_vec(&DebugCommand::Snapshot).unwrap(),
            &Authority::new([
                phenix_kernel::CapabilityId::parse("kernel.persistence.read").unwrap(),
            ]),
            None,
        ).unwrap();
        let DebugResponse::Snapshot { snapshot } = serde_json::from_slice(&output).unwrap();
        assert!(snapshot.services["sessions"].available);
        assert!(!snapshot.services["context"].available);
        assert!(!snapshot.services["models"].available);
    }
}
