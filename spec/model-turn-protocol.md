# Model turn protocol

Status: proposed

## Goal

Define one provider-neutral model contract for role-aware conversation, tool calls, tool results, streaming, and usage.

The agent loop must not parse provider-specific JSON or infer tool calls from text.

## Request

A model turn contains:

- selected model target or routing reference
- ordered instructions
- ordered conversation messages
- available tool definitions
- provider-neutral generation options

Prompt assembly produces instructions and context separately. The model request preserves that distinction instead of flattening every input into one byte string.

## Messages

Each message has an explicit role and ordered content parts.

Required roles:

- `user`
- `assistant`
- `tool`

Instructions are carried outside conversation history. Provider adapters map them to the strongest supported instruction role.

Content parts must support text and opaque binary media references without requiring every provider to support every part type. Unsupported required content fails before provider invocation.

## Tool definitions

A tool definition contains:

- stable callable id
- human-readable description
- structural input schema
- structural output schema when known
- concurrency declaration

Tool arguments and tool results use `PhenixValue`. JSON conversion belongs in provider adapters or external protocol adapters.

The model protocol does not grant tool authority. It only describes tools visible to the model. Invocation still passes through execution authority and tool policy.

## Response

A completed model turn contains ordered assistant output items. Each item is one of:

- assistant content
- tool call

A tool call contains:

- provider-neutral call id
- callable id
- structural arguments

Call ids are opaque within the session. The agent loop returns tool results against the exact call id.

The response also contains:

- finish reason
- token or provider usage when available
- provider metadata isolated from portable fields

## Finish reasons

Portable reasons:

- `complete`
- `tool_calls`
- `length`
- `content_filter`
- `cancelled`
- `provider_error`

Provider-specific reasons may be retained in metadata but do not replace the portable reason.

A response with tool calls uses `tool_calls` even if it also contains assistant text.

## Tool result turn

Each tool result contains:

- call id
- callable id
- structural result or typed failure

Tool failures are data returned to the model unless execution policy requires the whole agent run to fail. A tool panic or transport failure is never represented as successful tool output.

The next model turn receives tool results in the same order as the originating tool calls. Parallel execution must not reorder the conversation record.

## Streaming

Streaming is an optional transport behavior over the same logical turn.

Portable events:

1. assistant content delta
2. tool call started
3. tool argument delta when the provider exposes one
4. tool call completed
5. usage update
6. turn completed

Consumers may ignore deltas and use the final assembled response. Streaming must not change final semantics.

## Retained history

The session record needed for replay and compaction stores the provider-neutral logical turns, not raw provider requests.

Retained entries preserve:

- role
- content
- tool call ids and arguments
- tool results
- finish reason
- usage
- exact context and instruction revision references used to build the turn

Provider metadata may be retained separately for diagnostics. Replay correctness must not depend on it.

## Validation

Validate the request before provider invocation:

- tool ids are unique in one request
- content parts are supported by the selected provider target

Validate assistant tool calls before tool invocation:

- each tool call references a declared tool
- tool arguments satisfy the consumer tool schema

Validate tool results before the next provider invocation:

- each tool result references a pending call exactly once

Structural mismatches return errors and emit the existing mismatch event path. They do not panic.

## Provider adapters

A provider adapter owns:

- mapping portable roles to provider roles
- tool schema conversion
- provider request serialization
- provider response parsing
- streaming event assembly
- portable finish reason mapping
- provider-specific metadata

The core agent loop owns none of those mappings.

## Non-goals

- Define provider authentication.
- Define tool scheduling policy.
- Define compaction summarization.
- Preserve raw provider payloads as the authoritative session history.
- Require every provider to support every content type.

## Implementation order

1. Replace byte-only model input and output with the typed turn contract.
2. Adapt native providers and the basic model fixture.
3. Persist portable model and tool history in sessions.
4. Wire the agent loop to tool calls and tool results.
5. Add compaction against retained portable history.
