use super::*;

record!(SessionCreateInput, "phenix.application.type.session-create-input@1", {
    working_directory: String,
    title: Option<String>,
});
record!(SessionInput, "phenix.application.type.session-input@1", { session_id: SessionId });
record!(SessionRenameInput, "phenix.application.type.session-rename-input@1", {
    session_id: SessionId,
    title: String,
});
record!(PageInput, "phenix.application.type.page-input@1", { cursor: Option<String> });
record!(SessionInfo, "phenix.application.type.session-info@1", {
    session_id: SessionId,
    title: Option<String>,
    working_directory: String,
});
record!(SessionList, "phenix.application.type.session-list@1", {
    sessions: Vec<SessionInfo>,
    next_cursor: Option<String>,
});
record!(SessionLineage, "phenix.application.type.session-lineage@1", {
    session_id: SessionId,
    parent: Option<SessionId>,
    children: Vec<SessionId>,
});
record!(SessionResumeInput, "phenix.application.type.session-resume-input@1", {
    session_id: SessionId,
    after_sequence: Option<u64>,
});
record!(SessionSnapshot, "phenix.application.type.session-snapshot@1", {
    session: SessionInfo,
    through_sequence: u64,
    updates: Vec<SessionUpdate>,
});
variants!(MessageRole, "phenix.application.type.message-role@1", { User, Assistant });
variants!(Content, "phenix.application.type.content@1", {
    Text { text: String },
    Image { mime_type: String, data: phenix_core::Bytes },
    Resource { uri: String, mime_type: Option<String>, text: Option<String> },
});
record!(Message, "phenix.application.type.message@1", {
    role: MessageRole,
    content: Vec<Content>,
});
record!(PromptInput, "phenix.application.type.prompt-input@1", {
    session_id: SessionId,
    content: Vec<Content>,
});
variants!(StopReason, "phenix.application.type.stop-reason@1", {
    EndTurn, Cancelled, MaxTokens, Refused,
});
record!(PromptResult, "phenix.application.type.prompt-result@1", {
    execution_id: String,
    stop_reason: StopReason,
});
record!(ExecutionInput, "phenix.application.type.execution-input@1", {
    session_id: SessionId,
    execution_id: String,
});
variants!(ExecutionState, "phenix.application.type.execution-state@1", {
    Pending, Running, Completed, Cancelled, Failed { error: ApplicationError },
});
record!(ExecutionInfo, "phenix.application.type.execution-info@1", {
    execution_id: String,
    parent: Option<String>,
    state: ExecutionState,
});
record!(ExecutionTree, "phenix.application.type.execution-tree@1", {
    session_id: SessionId,
    executions: Vec<ExecutionInfo>,
});
record!(Provenance, "phenix.application.type.provenance@1", {
    execution_id: String,
    model_id: Option<ModelId>,
    routing_profile: Option<RoutingProfileId>,
    inputs: Vec<String>,
    outputs: Vec<String>,
});
