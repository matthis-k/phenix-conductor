# Observability

Status: proposed

## Goal

Define one structured diagnostic model for executions, model turns, tools, plugins, processes, and failures.

Observability must help debug the harness without making remote telemetry or sensitive payload capture a default behavior.

## Defaults

- Structured local events are enabled.
- Remote export is disabled.
- Prompt text, tool arguments, tool results, file contents, environment values, and secrets are excluded from telemetry fields by default.
- Identifiers needed to correlate runtime activity are included.
- Export plugins receive only the fields allowed by observability policy.
- Enabling an exporter does not grant network authority. The exporter still needs the corresponding capability.

## Correlation

Every observable operation carries the identifiers that exist for that operation:

- session id
- execution id
- parent execution id
- graph generation
- plugin id
- component or interface id
- callable id
- model target
- tool call id
- process id owned by the runtime abstraction

Identifiers are stable enough to correlate one run. They need not be globally identifying across installations unless their owning contract already requires that.

## Event envelope

Every event contains:

- event type and schema version
- timestamp
- severity
- correlation fields
- outcome when the event completes an operation
- typed event data

Event payload schemas are versioned. Exporters consume the typed envelope rather than parsing log text.

## Core event families

### Execution

Record:

- created
- delegated
- started
- completed
- failed
- cancelled

Include effective authority identity or a non-secret capability summary. Do not copy arbitrary authority payloads into free-form text.

### Model

Record:

- routing decision
- request started
- first response event when streaming
- request completed
- request failed
- portable finish reason
- token and cache usage when available
- latency

Prompt content is referenced by retained-history or context revision ids when available. It is not duplicated into telemetry.

### Tool

Record:

- tool selected
- invocation started
- invocation completed
- invocation failed
- concurrency class
- queue and execution duration

Arguments and results are omitted by default. Diagnostics may include structural schema identities, byte sizes, and validation failures.

### Process

Record:

- confinement resolution
- process started
- process completed
- process cancelled
- execution limit exceeded
- output truncation

Do not include command environment values. Command text is diagnostic content and follows explicit content-capture policy rather than ordinary metadata policy.

### Plugin

Record:

- discovered
- graph resolved
- activated
- stopped
- replaced
- failed
- structural ABI mismatch

Graph events identify the generation so dynamic reload behavior can be reconstructed.

## Metrics

Metrics are derived from structured events where practical. Avoid a second mutable accounting path when the event stream already contains the required facts.

Useful default metrics:

- active executions
- execution duration
- model request count and latency
- input, output, and cached tokens
- tool invocation count and latency
- process count and duration
- plugin activation failures
- structural ABI mismatches
- cancellation count

Metric labels must remain bounded. Session ids, execution ids, paths, prompts, and tool arguments are not metric labels.

## Logs

Human-readable logs are a rendering of structured events plus explicit diagnostic messages.

Libraries do not print directly to stdout or stderr as their observability contract. Frontends and host applications decide where rendered logs go.

## Content capture

Content capture is a separate policy from observability enablement.

Default: `metadata_only`.

Potential explicit modes:

- `metadata_only`
- `diagnostic_content`

`diagnostic_content` may capture prompts, tool arguments, tool results, command text, or model output only for fields explicitly allowed by policy. Secret values remain excluded.

Content capture should support per-execution scope so a user can debug one run without changing the global default.

## Export

Remote export is provided by plugins. OpenTelemetry is the preferred interoperability target for traces and metrics, but the core contract does not depend on one transport.

An exporter must:

- declare required network authority
- declare which event schemas it accepts
- receive already-filtered fields
- fail without blocking the core execution path unless policy marks export as required

The default Phenix configuration has no remote exporter.

## Retention

Local retention policy belongs to the selected observability storage plugin.

The default should keep bounded recent diagnostics rather than an unbounded durable event archive. Domain state and replay journals remain separate from observability data.

Deleting observability data must not make domain replay invalid.

## Failures

Observability failures do not normally fail an agent execution.

Exceptions:

- a required audit exporter configured by deployment policy fails
- the runtime cannot record an event that is itself required for a security or durable audit contract

Those requirements must be explicit. Ordinary debug telemetry remains best effort.

## Non-goals

- Use telemetry as authoritative domain persistence.
- Enable remote reporting by default.
- Store full prompts or tool payloads by default.
- Put unbounded identifiers into metric labels.
- Require OpenTelemetry inside core types.

## Implementation order

1. Define the event envelope and core event families.
2. Route current structural mismatch and lifecycle diagnostics through the envelope.
3. Add execution, model, tool, and process events.
4. Add bounded local storage and debug rendering.
5. Add an optional OpenTelemetry exporter plugin.
6. Add explicit diagnostic content-capture policy.
