#[derive(Clone, Debug)]
struct SessionRecord {
    summary: SessionSummary,
}

#[derive(Clone, Debug)]
enum ExecutionPayload {
    Invocation { input: String },
    Orchestration { input: Value },
}

#[derive(Clone, Debug)]
struct ExecutionRecord {
    summary: ExecutionSummary,
    payload: ExecutionPayload,
    authority: ExecutionAuthority,
    config_revision: ConfigRevisionId,
}
