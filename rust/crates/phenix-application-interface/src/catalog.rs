use crate::{descriptor::id, types::*, *};
use phenix_core::PhenixContract;
use std::collections::{BTreeMap, BTreeSet};

macro_rules! operations {
    ($($name:ident: $operation:literal, $capability:literal, $input:ty => $output:ty;)*) => {
        $(pub struct $name;
        impl Operation for $name {
            const ID: &'static str = concat!("phenix.application.", $operation, "@1");
            const CAPABILITY: &'static str = concat!("phenix.application.capability.", $capability, "@1");
            type Input = $input;
            type Output = $output;
        })*

        fn register_operations(descriptor: &mut ApplicationDescriptor) {
            $(let input = descriptor.register::<$input>();
            let output = descriptor.register::<$output>();
            descriptor.operations.insert(id($name::ID), OperationDescriptor {
                input,
                output,
                error: ApplicationError::contract_id(),
                capability: id($name::CAPABILITY),
            });)*
        }
    };
}

operations! {
    Discover: "capabilities", "discovery", Empty => CapabilityList;
    DiscoverAuthentication: "authentication-list", "authentication", Empty => AuthenticationMethods;
    Authenticate: "authenticate", "authentication", AuthenticateInput => AuthenticationResult;
    CreateSession: "session-create", "sessions", SessionCreateInput => SessionInfo;
    ListSessions: "session-list", "session-list", PageInput => SessionList;
    ResumeSession: "session-resume", "session-resume", SessionResumeInput => SessionSnapshot;
    RenameSession: "session-rename", "session-rename", SessionRenameInput => SessionInfo;
    CloseSession: "session-close", "sessions", SessionInput => Acknowledged;
    GetLineage: "session-lineage", "lineage", SessionInput => SessionLineage;
    Prompt: "prompt", "prompt", PromptInput => PromptResult;
    Cancel: "cancel", "prompt", SessionInput => Acknowledged;
    ListModels: "model-list", "models", SessionInput => Models;
    SelectModel: "model-select", "models", ModelSelectInput => Models;
    ListRoutingProfiles: "routing-list", "routing", SessionInput => RoutingProfiles;
    SelectRoutingProfile: "routing-select", "routing", RoutingSelectInput => RoutingProfiles;
    ListSkills: "skill-list", "skills", SessionInput => Skills;
    ActivateSkill: "skill-activate", "skills", SkillActivateInput => Skills;
    ListCallables: "callable-list", "callables", SessionInput => Callables;
    InvokeCallable: "callable-invoke", "callables", CallableInvokeInput => CallableResult;
    GetExecutionTree: "execution-tree", "inspection", SessionInput => ExecutionTree;
    GetProvenance: "execution-provenance", "inspection", ExecutionInput => Provenance;
    GetDiagnostics: "diagnostics", "diagnostics", Empty => Diagnostics;
}

/// Describes all application features. Connected runtimes advertise only implemented features.
/// In particular, this catalog is never an implicit runtime capability advertisement.
#[must_use]
pub fn application_descriptor() -> ApplicationDescriptor {
    let mut descriptor = ApplicationDescriptor {
        id: id(INTERFACE_ID),
        operations: BTreeMap::new(),
        events: BTreeMap::new(),
        callbacks: BTreeMap::new(),
        capabilities: BTreeMap::new(),
        types: BTreeMap::new(),
    };
    descriptor.register::<ApplicationError>();
    register_operations(&mut descriptor);
    macro_rules! named_types {
        ($($ty:ty),* $(,)?) => { $(descriptor.register::<$ty>();)* };
    }
    named_types!(
        AuthenticationMethod,
        ModelInfo,
        RoutingInfo,
        SkillInfo,
        CallableInfo,
        Diagnostic,
        Severity,
        MessageRole,
        Content,
        Message,
        StopReason,
        ExecutionState,
        ExecutionInfo,
        SessionChange,
        ExecutionChange,
    );
    for (name, dependencies) in [
        ("discovery", vec![]),
        ("authentication", vec!["discovery"]),
        ("sessions", vec!["discovery"]),
        ("session-list", vec!["sessions"]),
        ("session-resume", vec!["sessions"]),
        ("session-rename", vec!["sessions"]),
        ("lineage", vec!["sessions"]),
        ("prompt", vec!["sessions"]),
        ("models", vec!["sessions"]),
        ("routing", vec!["sessions"]),
        ("skills", vec!["sessions"]),
        ("callables", vec!["sessions"]),
        ("inspection", vec!["sessions"]),
        ("diagnostics", vec!["discovery"]),
        ("permission", vec!["prompt"]),
        ("elicitation", vec!["sessions"]),
        ("client-callables", vec!["callables"]),
    ] {
        descriptor.capabilities.insert(
            capability(name),
            CapabilityDescriptor {
                dependencies: dependencies
                    .into_iter()
                    .map(capability)
                    .collect::<BTreeSet<_>>(),
            },
        );
    }
    let payload = descriptor.register::<SessionUpdate>();
    descriptor.events.insert(
        id("phenix.application.session-update@1"),
        EventDescriptor {
            payload,
            ordering: OrderingScope::Session,
            capability: capability("sessions"),
        },
    );
    let payload = descriptor.register::<ExecutionUpdate>();
    descriptor.events.insert(
        id("phenix.application.execution-update@1"),
        EventDescriptor {
            payload,
            ordering: OrderingScope::Execution,
            capability: capability("prompt"),
        },
    );
    macro_rules! callback {
        ($name:literal, $capability:literal, $request:ty, $response:ty, $semantics:ident) => {
            let request = descriptor.register::<$request>();
            let response = descriptor.register::<$response>();
            descriptor.callbacks.insert(
                id(concat!("phenix.application.", $name, "@1")),
                CallbackDescriptor {
                    request,
                    response,
                    capability: capability($capability),
                    semantics: CallbackSemantics::$semantics,
                },
            );
        };
    }
    callback!(
        "permission",
        "permission",
        PermissionRequest,
        PermissionResponse,
        InvocationConsent
    );
    callback!(
        "elicitation",
        "elicitation",
        ElicitationRequest,
        ElicitationResponse,
        Data
    );
    callback!(
        "client-callable",
        "client-callables",
        ClientCallableRequest,
        ClientCallableResponse,
        InvocationConsent
    );
    descriptor
}

fn capability(name: &str) -> phenix_core::ContractId {
    id(&format!("phenix.application.capability.{name}@1"))
}
