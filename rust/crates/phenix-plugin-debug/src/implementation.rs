use crate::{
    ContextProbeCommand, FrontendProbeCommand, JobProbeCommand, ModelProbeCommand,
    PlanningProbeCommand, SessionProbeCommand,
};
use phenix_core::{
    Authority, ComponentInterface, ComponentInvocationError, PhenixValue, PluginExecution,
    PluginHost, PluginInstance, PluginManifest, SdkClient, ServiceContribution, ServiceId,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const DEBUG_SERVICE: &str = "phenix.debug@1";

phenix_core::phenix_plugin! {
    "phenix.debug";

    uses {
        sessions: "phenix.sessions@1",
        context: "phenix.context@1",
        planning: "phenix.planning@1",
        jobs: "phenix.jobs@1",
        models: "phenix.models.routing@1",
        frontends: "phenix.frontend-services@1",
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, phenix_sdk_macros::PhenixValue)]
#[serde(tag = "operation", rename_all = "snake_case")]
pub enum DebugCommand {
    Snapshot,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, phenix_sdk_macros::PhenixValue)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum DiagnosticEntry {
    Available { value: serde_json::Value },
    Unavailable { error: String },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, phenix_sdk_macros::PhenixValue)]
pub struct DiagnosticSnapshot {
    pub services: BTreeMap<String, DiagnosticEntry>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, phenix_sdk_macros::PhenixValue)]
#[serde(tag = "response", rename_all = "snake_case")]
pub enum DebugResponse {
    Snapshot { snapshot: DiagnosticSnapshot },
}

#[must_use]
pub fn debug_manifest(maximum_authority: Authority) -> PluginManifest {
    PluginManifest {
        id: phenix_plugin::plugin_id(),
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
    ServiceId::parse(DEBUG_SERVICE).expect("static debug service id is valid")
}

type DebugContext<'host, 'runtime> = phenix_plugin::Context<'host, 'runtime>;

fn context<'host, 'runtime>(host: &'host PluginHost<'runtime>) -> DebugContext<'host, 'runtime> {
    phenix_plugin::context(host, (), ())
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
        let context = context(host);
        let interface = crate::DebugInterface::interface_id();
        let command = context
            .kernel
            .decode_projected::<DebugCommand>(&interface, input)
            .map_err(|error| error.to_string())?;
        let response = handle(&context, command);
        context
            .kernel
            .encode_value(&response)
            .map_err(|error| error.to_string())
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
        &SessionProbeCommand::List,
    );
    probe(
        &context.sdk.context,
        &mut services,
        "context",
        &ContextProbeCommand::List,
    );
    probe(
        &context.sdk.planning,
        &mut services,
        "planning_history",
        &PlanningProbeCommand::SearchHistory {
            objective_id: None,
            query: String::new(),
        },
    );
    probe(
        &context.sdk.jobs,
        &mut services,
        "jobs",
        &JobProbeCommand::List,
    );
    probe(
        &context.sdk.models,
        &mut services,
        "models",
        &ModelProbeCommand::ListProfiles,
    );
    probe(
        &context.sdk.frontends,
        &mut services,
        "frontends",
        &FrontendProbeCommand::Catalog,
    );
    DiagnosticSnapshot { services }
}

fn probe<I, Request>(
    client: &SdkClient<'_, '_, I>,
    entries: &mut BTreeMap<String, DiagnosticEntry>,
    name: &str,
    command: &Request,
) where
    I: ComponentInterface,
    for<'value> PhenixValue: From<&'value Request>,
{
    let request = PhenixValue::from(command);
    let entry = match client.invoke_value(&request) {
        Ok(response) => response_entry(response),
        Err(error) => error_entry(error),
    };
    entries.insert(name.into(), entry);
}

fn response_entry(response: PhenixValue) -> DiagnosticEntry {
    match serde_json::to_value(response) {
        Ok(value) => DiagnosticEntry::Available { value },
        Err(error) => DiagnosticEntry::Unavailable {
            error: format!("invalid diagnostic service response: {error}"),
        },
    }
}

fn error_entry(error: ComponentInvocationError) -> DiagnosticEntry {
    DiagnosticEntry::Unavailable {
        error: error.to_string(),
    }
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

        let input = serde_json::to_vec(&PhenixValue::from(&DebugCommand::Snapshot)).unwrap();
        let output = kernel
            .invoke(&debug_service(), &input, &authority, None)
            .unwrap();
        let output: PhenixValue = serde_json::from_slice(&output).unwrap();
        let DebugResponse::Snapshot { snapshot } = output.project().unwrap();
        assert!(matches!(
            snapshot.services["sessions"],
            DiagnosticEntry::Available { .. }
        ));
        assert!(matches!(
            snapshot.services["context"],
            DiagnosticEntry::Unavailable { .. }
        ));
        assert!(matches!(
            snapshot.services["models"],
            DiagnosticEntry::Unavailable { .. }
        ));
    }
}
