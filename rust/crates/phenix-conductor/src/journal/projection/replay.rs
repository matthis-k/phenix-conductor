use super::state::DurableProjection;
use crate::ExecutionPayload;
use phenix_core::{
    ExecutionEventKind, ExecutionId, ExecutionKind, ExecutionSummary, ExecutionTarget,
};
use serde::Serialize;

use crate::journal::JournalExecutionPayload;

#[derive(Serialize)]
struct ConversationReplayMessage {
    role: &'static str,
    content: String,
}

struct AccumulatedMessage {
    execution_id: ExecutionId,
    role: &'static str,
    content: String,
}

pub(super) fn materialize_execution_payload(
    state: &DurableProjection<'_>,
    execution: &ExecutionSummary,
    payload: &JournalExecutionPayload,
) -> ExecutionPayload {
    match payload {
        JournalExecutionPayload::Invocation { input, .. }
            if execution.kind == ExecutionKind::Root
                && matches!(execution.target, ExecutionTarget::Routed(_)) =>
        {
            ExecutionPayload::Invocation {
                input: materialize_routed_input(state, execution, input),
            }
        }
        _ => payload.clone().into(),
    }
}

fn materialize_routed_input(
    state: &DurableProjection<'_>,
    execution: &ExecutionSummary,
    input: &str,
) -> String {
    let mut messages = Vec::<AccumulatedMessage>::new();

    for event in state.events.iter() {
        if event.session_id != execution.session_id || event.execution_id == execution.id {
            continue;
        }
        let Some(previous) = state.executions.get(&event.execution_id) else {
            continue;
        };
        if previous.summary.kind != ExecutionKind::Root
            || previous.summary.parent_execution.is_some()
        {
            continue;
        }

        match &event.kind {
            ExecutionEventKind::UserInput { text } => messages.push(AccumulatedMessage {
                execution_id: event.execution_id.clone(),
                role: "user",
                content: text.clone(),
            }),
            ExecutionEventKind::AssistantContentDelta { text } => {
                if let Some(last) = messages.last_mut().filter(|message| {
                    message.execution_id == event.execution_id && message.role == "assistant"
                }) {
                    last.content.push_str(text);
                } else {
                    messages.push(AccumulatedMessage {
                        execution_id: event.execution_id.clone(),
                        role: "assistant",
                        content: text.clone(),
                    });
                }
            }
            _ => {}
        }
    }

    if messages.is_empty() {
        return input.to_owned();
    }

    let replay = messages
        .into_iter()
        .map(|message| ConversationReplayMessage {
            role: message.role,
            content: message.content,
        })
        .collect::<Vec<_>>();
    let replay = serde_json::to_string(&replay)
        .expect("conversation replay contains only JSON-serializable strings");

    format!(
        "Continue the same Phenix conversation. The prior user/assistant messages are serialized as JSON in chronological order. Treat each entry according to its `role`, then answer the current user message.\n\nPrior conversation:\n{replay}\n\nCurrent user message:\n{input}"
    )
}
