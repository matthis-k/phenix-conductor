use super::*;

record!(CapabilityList, "phenix.application.type.capability-list@1", {
    interface: ContractId,
    capabilities: Vec<ContractId>,
});
record!(AuthenticationMethod, "phenix.application.type.authentication-method@1", {
    id: String,
    name: String,
    description: Option<String>,
});
record!(AuthenticationMethods, "phenix.application.type.authentication-methods@1", {
    methods: Vec<AuthenticationMethod>,
});
record!(AuthenticateInput, "phenix.application.type.authenticate-input@1", { method_id: String });
variants!(AuthenticationResult, "phenix.application.type.authentication-result@1", {
    Authenticated,
    External { uri: String, instructions: Option<String> },
});
record!(ModelInfo, "phenix.application.type.model-info@1", {
    id: ModelId,
    name: String,
    description: Option<String>,
});
record!(Models, "phenix.application.type.models@1", {
    available: Vec<ModelInfo>,
    selected: Option<ModelId>,
});
record!(ModelSelectInput, "phenix.application.type.model-select-input@1", {
    session_id: SessionId,
    model_id: ModelId,
});
record!(RoutingInfo, "phenix.application.type.routing-info@1", {
    id: RoutingProfileId,
    name: String,
});
record!(RoutingProfiles, "phenix.application.type.routing-profiles@1", {
    available: Vec<RoutingInfo>,
    selected: Option<RoutingProfileId>,
});
record!(RoutingSelectInput, "phenix.application.type.routing-select-input@1", {
    session_id: SessionId,
    profile_id: RoutingProfileId,
});
record!(SkillInfo, "phenix.application.type.skill-info@1", {
    id: SkillId,
    name: String,
    description: String,
    active: bool,
});
record!(Skills, "phenix.application.type.skills@1", { skills: Vec<SkillInfo> });
record!(SkillActivateInput, "phenix.application.type.skill-activate-input@1", {
    session_id: SessionId,
    skill_id: SkillId,
});
record!(CallableInfo, "phenix.application.type.callable-info@1", {
    id: CallableId,
    description: String,
    input: PhenixSchema,
    output: PhenixSchema,
});
record!(Callables, "phenix.application.type.callables@1", { callables: Vec<CallableInfo> });
record!(CallableInvokeInput, "phenix.application.type.callable-invoke-input@1", {
    session_id: SessionId,
    callable_id: CallableId,
    input: PhenixValue,
});
record!(CallableResult, "phenix.application.type.callable-result@1", { output: PhenixValue });
variants!(Severity, "phenix.application.type.severity@1", { Info, Warning, Error });
record!(Diagnostic, "phenix.application.type.diagnostic@1", {
    code: String,
    severity: Severity,
    message: String,
    resource: Option<String>,
});
record!(Diagnostics, "phenix.application.type.diagnostics@1", { diagnostics: Vec<Diagnostic> });
