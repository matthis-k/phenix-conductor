use super::*;

// Sequence numbers increase within the declared scope. Resume snapshots include their watermark.
record!(SessionUpdate, "phenix.application.type.session-update@1", {
    session_id: SessionId,
    sequence: u64,
    update: SessionChange,
});
variants!(SessionChange, "phenix.application.type.session-change@1", {
    Message { message: Message },
    TextDelta { execution_id: String, text: String },
    Renamed { title: String },
    Closed,
    Execution { execution_id: String, update: ExecutionChange },
    Diagnostic { diagnostic: Diagnostic },
});
record!(ExecutionUpdate, "phenix.application.type.execution-update@1", {
    session_id: SessionId,
    execution_id: String,
    sequence: u64,
    update: ExecutionChange,
});
variants!(ExecutionChange, "phenix.application.type.execution-change@1", {
    State { state: ExecutionState },
    ToolCall { call_id: String, callable_id: CallableId, input: PhenixValue },
    ToolResult { call_id: String, output: PhenixValue },
    ToolFailed { call_id: String, error: ApplicationError },
    Progress { message: String, fraction: Option<f64> },
});
record!(PermissionRequest, "phenix.application.type.permission-request@1", {
    session_id: SessionId,
    execution_id: String,
    call_id: String,
    description: String,
});
variants!(PermissionResponse, "phenix.application.type.permission-response@1", {
    AllowOnce, Deny, Cancelled,
});
record!(ElicitationRequest, "phenix.application.type.elicitation-request@1", {
    session_id: SessionId,
    message: String,
    schema: PhenixSchema,
});
variants!(ElicitationResponse, "phenix.application.type.elicitation-response@1", {
    Accepted { value: PhenixValue }, Declined, Cancelled,
});
record!(ClientCallableRequest, "phenix.application.type.client-callable-request@1", {
    session_id: SessionId,
    execution_id: String,
    call_id: String,
    callable_id: CallableId,
    input: PhenixValue,
});
variants!(ClientCallableResponse, "phenix.application.type.client-callable-response@1", {
    Completed { output: PhenixValue }, Failed { error: ApplicationError },
});
