use super::{
    language_frontend::FrontendLanguageServices, managed_language::ManagedLanguageProviders,
    FrontendServiceRouter,
};
use crate::{CompiledConfiguration, ConductorError, ToolOutcome};
use phenix_core::{
    ActiveLanguageProvider, CallableDescriptor, CallableId, CallableKind, CallablePolicy,
    CapabilitySet, LanguageDocumentProvenance, LanguageObservationInput, LanguageOperation,
    LanguageOperationResult, LanguagePosition, LanguageProviderId, LanguageProviderSource,
    LanguageServiceConfiguration, LanguageServiceKind, WorkspaceId, CAPABILITY_FILESYSTEM_READ,
};
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

const FRONTEND_LANGUAGE_EXECUTE: &str = "language.execute";

#[derive(Clone)]
pub(super) struct LanguageToolRuntime {
    workspace_root: Option<PathBuf>,
    frontend_services: FrontendServiceRouter,
    language_services: FrontendLanguageServices,
    managed_language: ManagedLanguageProviders,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DocumentPositionInput {
    service: LanguageServiceKind,
    document: PathBuf,
    line: u32,
    character: u32,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DocumentInput {
    service: LanguageServiceKind,
    document: PathBuf,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct WorkspaceSymbolsInput {
    service: LanguageServiceKind,
    query: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DiagnosticsInput {
    service: LanguageServiceKind,
    document: Option<PathBuf>,
}

impl LanguageToolRuntime {
    pub(super) fn new(
        workspace_root: Option<PathBuf>,
        frontend_services: FrontendServiceRouter,
        language_services: FrontendLanguageServices,
        managed_language: ManagedLanguageProviders,
    ) -> Self {
        Self {
            workspace_root,
            frontend_services,
            language_services,
            managed_language,
        }
    }

    pub(super) fn register_into(
        &self,
        configuration: &mut CompiledConfiguration,
    ) -> Result<(), ConductorError> {
        for (id, description, schema) in document_position_tools() {
            let runtime = self.clone();
            let id_for_handler = id.clone();
            configuration.register_contextual_tool(
                descriptor(id, description, schema),
                move |context, arguments| {
                    let input: DocumentPositionInput = serde_json::from_str(arguments)
                        .map_err(|error| format!("invalid language tool arguments: {error}"))?;
                    let position = LanguagePosition {
                        line: input.line,
                        character: input.character,
                    };
                    let operation = match id_for_handler.as_str() {
                        "phenix_lsp_definition" => LanguageOperation::Definition {
                            document: input.document,
                            position,
                        },
                        "phenix_lsp_references" => LanguageOperation::References {
                            document: input.document,
                            position,
                        },
                        "phenix_lsp_implementations" => LanguageOperation::Implementations {
                            document: input.document,
                            position,
                        },
                        "phenix_lsp_hover" => LanguageOperation::Hover {
                            document: input.document,
                            position,
                        },
                        "phenix_lsp_call_hierarchy" => LanguageOperation::CallHierarchy {
                            document: input.document,
                            position,
                        },
                        _ => return Err("unknown language operation".to_owned()),
                    };
                    runtime.invoke(context, input.service, operation)
                },
            )?;
        }

        let runtime = self.clone();
        configuration.register_contextual_tool(
            descriptor(
                CallableId::parse("phenix_lsp_document_symbols").expect("static callable id"),
                "List language symbols in one workspace document.",
                document_schema(),
            ),
            move |context, arguments| {
                let input: DocumentInput = serde_json::from_str(arguments)
                    .map_err(|error| format!("invalid language tool arguments: {error}"))?;
                runtime.invoke(
                    context,
                    input.service,
                    LanguageOperation::DocumentSymbols {
                        document: input.document,
                    },
                )
            },
        )?;

        let runtime = self.clone();
        configuration.register_contextual_tool(
            descriptor(
                CallableId::parse("phenix_lsp_workspace_symbols").expect("static callable id"),
                "Search language symbols across the workspace.",
                workspace_symbols_schema(),
            ),
            move |context, arguments| {
                let input: WorkspaceSymbolsInput = serde_json::from_str(arguments)
                    .map_err(|error| format!("invalid language tool arguments: {error}"))?;
                runtime.invoke(
                    context,
                    input.service,
                    LanguageOperation::WorkspaceSymbols { query: input.query },
                )
            },
        )?;

        let runtime = self.clone();
        configuration.register_contextual_tool(
            descriptor(
                CallableId::parse("phenix_lsp_diagnostics").expect("static callable id"),
                "Read current language diagnostics for the workspace or one document.",
                diagnostics_schema(),
            ),
            move |context, arguments| {
                let input: DiagnosticsInput = serde_json::from_str(arguments)
                    .map_err(|error| format!("invalid language tool arguments: {error}"))?;
                runtime.invoke(
                    context,
                    input.service,
                    LanguageOperation::Diagnostics {
                        document: input.document,
                    },
                )
            },
        )?;
        Ok(())
    }

    fn invoke(
        &self,
        context: &crate::callables::ToolExecutionContext,
        service: LanguageServiceKind,
        operation: LanguageOperation,
    ) -> Result<ToolOutcome, String> {
        let observation = self.execute(
            &context.execution_id,
            &context.workspace_id,
            &context.language_configuration,
            &service,
            operation,
        )?;
        let output = serde_json::to_string(&observation.result)
            .map_err(|error| format!("cannot encode language result: {error}"))?;
        Ok(ToolOutcome::success(output).with_language_observation(observation))
    }

    pub(super) fn execute(
        &self,
        execution: &phenix_core::ExecutionId,
        workspace: &WorkspaceId,
        configuration: &LanguageServiceConfiguration,
        service: &LanguageServiceKind,
        operation: LanguageOperation,
    ) -> Result<LanguageObservationInput, String> {
        let active = self
            .active_provider(workspace, service, configuration)?
            .ok_or_else(|| format!("language service {service} is unavailable"))?;
        let result = match &active.source {
            LanguageProviderSource::Frontend { .. } => {
                if matches!(operation, LanguageOperation::Diagnostics { .. })
                    && active.capabilities.shared_diagnostics
                {
                    if let Some(result) = self
                        .language_services
                        .diagnostics(workspace, service)
                        .map_err(|error| error.to_string())?
                    {
                        validate_frontend_result(&active, &result)?;
                        result
                    } else {
                        self.request_frontend_operation(
                            execution,
                            workspace,
                            service,
                            configuration,
                            &active,
                            &operation,
                        )?
                    }
                } else {
                    self.request_frontend_operation(
                        execution,
                        workspace,
                        service,
                        configuration,
                        &active,
                        &operation,
                    )?
                }
            }
            LanguageProviderSource::Managed { generation } => {
                let (actual_generation, result) = self
                    .managed_language
                    .request(service, &active.provider, &operation)
                    .map_err(|error| error.to_string())?;
                if actual_generation != *generation {
                    return Err(
                        "language provider changed while the request was running".to_owned()
                    );
                }
                result
            }
        };
        let after = self
            .active_provider(workspace, service, configuration)?
            .ok_or_else(|| "language provider changed while the request was running".to_owned())?;
        if after.provider != active.provider
            || after.epoch != active.epoch
            || after.source != active.source
        {
            return Err("language provider changed while the request was running".to_owned());
        }
        Ok(LanguageObservationInput {
            execution: execution.clone(),
            workspace: workspace.clone(),
            service: service.clone(),
            provider: active.provider,
            provider_epoch: active.epoch,
            operation,
            result,
        })
    }

    fn request_frontend_operation(
        &self,
        _execution: &phenix_core::ExecutionId,
        workspace: &WorkspaceId,
        service: &LanguageServiceKind,
        configuration: &LanguageServiceConfiguration,
        active: &ActiveLanguageProvider,
        operation: &LanguageOperation,
    ) -> Result<LanguageOperationResult, String> {
        let params = serde_json::to_value(operation)
            .map_err(|error| format!("cannot encode language operation: {error}"))?;
        let value = self
            .language_services
            .request(
                workspace,
                service,
                configuration,
                &self.frontend_services,
                FRONTEND_LANGUAGE_EXECUTE.to_owned(),
                params,
            )
            .map_err(|error| error.to_string())?;
        let result: LanguageOperationResult = serde_json::from_value(value)
            .map_err(|error| format!("invalid frontend language result: {error}"))?;
        validate_frontend_result(active, &result)?;
        Ok(result)
    }

    fn active_provider(
        &self,
        workspace: &WorkspaceId,
        service: &LanguageServiceKind,
        configuration: &LanguageServiceConfiguration,
    ) -> Result<Option<ActiveLanguageProvider>, String> {
        let live_managed = self.live_managed(workspace, service, configuration)?;
        self.language_services
            .reconcile(
                workspace,
                service,
                configuration,
                &self.frontend_services,
                &live_managed,
            )
            .map_err(|error| error.to_string())
    }

    fn live_managed(
        &self,
        workspace: &WorkspaceId,
        service: &LanguageServiceKind,
        configuration: &LanguageServiceConfiguration,
    ) -> Result<BTreeMap<LanguageProviderId, u64>, String> {
        let Some(root) = &self.workspace_root else {
            return Ok(BTreeMap::new());
        };
        let requirement = configuration.requirement_for(service);
        let mut definitions = configuration
            .managed_for(service)
            .into_iter()
            .filter(|definition| {
                definition
                    .capabilities
                    .satisfies(&requirement.required_capabilities)
            })
            .collect::<Vec<_>>();
        definitions.sort_by(|left, right| left.provider.cmp(&right.provider));
        let current = self
            .language_services
            .current(workspace, service)
            .map_err(|error| error.to_string())?;
        let selected = current
            .as_ref()
            .filter(|active| matches!(active.source, LanguageProviderSource::Managed { .. }))
            .and_then(|active| {
                definitions
                    .iter()
                    .find(|definition| definition.provider == active.provider)
            })
            .or_else(|| {
                requirement
                    .preferred_provider
                    .as_ref()
                    .and_then(|preferred| {
                        definitions
                            .iter()
                            .find(|definition| &definition.provider == preferred)
                    })
            });
        let selected = if selected.is_some() {
            selected
        } else if current
            .as_ref()
            .is_some_and(|active| matches!(active.source, LanguageProviderSource::Frontend { .. }))
            || self
                .language_services
                .has_eligible_frontend(service, configuration, &self.frontend_services)
                .map_err(|error| error.to_string())?
        {
            None
        } else {
            definitions.first()
        };
        match selected {
            Some(definition) => self
                .managed_language
                .ensure_definitions(root, [definition.clone()])
                .map_err(|error| error.to_string()),
            None => Ok(BTreeMap::new()),
        }
    }
}

fn validate_frontend_result(
    active: &ActiveLanguageProvider,
    result: &LanguageOperationResult,
) -> Result<(), String> {
    if !active.capabilities.dirty_buffers
        && result
            .documents
            .iter()
            .any(|document| document.provenance != LanguageDocumentProvenance::WorkspaceBacked)
    {
        return Err(format!(
            "frontend language provider {} returned non-workspace document state without dirty-buffer capability",
            active.provider
        ));
    }
    if result.documents.iter().any(|document| {
        document.provenance == LanguageDocumentProvenance::WorkspaceBacked
            && document.workspace_version.is_none()
    }) {
        return Err("workspace-backed language result omitted an exact file version".to_owned());
    }
    Ok(())
}

fn descriptor(id: CallableId, description: &str, input_schema: Value) -> CallableDescriptor {
    CallableDescriptor {
        id,
        kind: CallableKind::Tool,
        description: description.to_owned(),
        input_schema,
        output_schema: json!({"type": "object"}),
        capabilities: CapabilitySet(BTreeSet::from([CAPABILITY_FILESYSTEM_READ.to_owned()])),
        policy: CallablePolicy {
            requires_permission: false,
        },
    }
}

fn document_position_tools() -> Vec<(CallableId, &'static str, Value)> {
    [
        (
            "phenix_lsp_definition",
            "Find the definition at a workspace document position.",
        ),
        (
            "phenix_lsp_references",
            "Find references at a workspace document position.",
        ),
        (
            "phenix_lsp_implementations",
            "Find implementations at a workspace document position.",
        ),
        (
            "phenix_lsp_hover",
            "Read language hover information at a workspace document position.",
        ),
        (
            "phenix_lsp_call_hierarchy",
            "Read call hierarchy information at a workspace document position.",
        ),
    ]
    .into_iter()
    .map(|(id, description)| {
        (
            CallableId::parse(id).expect("static callable id"),
            description,
            document_position_schema(),
        )
    })
    .collect()
}

fn service_property() -> Value {
    json!({"type": "string", "minLength": 1})
}

fn document_position_schema() -> Value {
    json!({
        "type": "object",
        "additionalProperties": false,
        "required": ["service", "document", "line", "character"],
        "properties": {
            "service": service_property(),
            "document": {"type": "string", "minLength": 1},
            "line": {"type": "integer", "minimum": 0},
            "character": {"type": "integer", "minimum": 0}
        }
    })
}

fn document_schema() -> Value {
    json!({
        "type": "object",
        "additionalProperties": false,
        "required": ["service", "document"],
        "properties": {
            "service": service_property(),
            "document": {"type": "string", "minLength": 1}
        }
    })
}

fn workspace_symbols_schema() -> Value {
    json!({
        "type": "object",
        "additionalProperties": false,
        "required": ["service", "query"],
        "properties": {
            "service": service_property(),
            "query": {"type": "string"}
        }
    })
}

fn diagnostics_schema() -> Value {
    json!({
        "type": "object",
        "additionalProperties": false,
        "required": ["service"],
        "properties": {
            "service": service_property(),
            "document": {"type": "string", "minLength": 1}
        }
    })
}
