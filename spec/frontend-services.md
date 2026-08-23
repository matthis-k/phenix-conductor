# Frontend service providers

## Purpose

Some capabilities live in a frontend process rather than the conductor. Examples include browser-backed services and editor-owned integrations. A frontend service provider lets conductor-owned logic use those capabilities over an existing frontend connection.

This interface is process-local. It does not change Phenix session identity, configuration revisions, callable ownership, or execution authority.

## Lifecycle

A frontend advertises its current providers with the connection-level `set_frontend_service_providers` envelope. The envelope replaces the provider set for that connection. It is connection control, not a conductor `Command`.

```json
{
  "type": "set_frontend_service_providers",
  "id": 4,
  "providers": [
    {
      "id": "web",
      "capabilities": ["fetch", "search"]
    }
  ]
}
```

The frontend may advertise again when availability changes. An empty list removes all providers from the connection.

Disconnect removes the provider set, execution routes, and pending calls immediately. A conductor restart starts with no frontend providers. Registrations are never written to SQLite or reconstructed from the journal.

The conductor can inspect the live catalog with each descriptor's source connection identity and capabilities. This lets conductor-owned workspace services select one eligible frontend without inventing durable frontend ownership.

## Execution routing

A frontend connection does not own a session. Multiple frontends may submit to the same session.

When the conductor accepts a root execution from a frontend connection, it binds that root to the connection before the execution becomes runnable. Descendant service calls resolve the execution's durable `parent_execution` chain to that root.

```text
frontend A -> root A -> child A1 -> child A2
                  |
                  +-> execution-scoped service calls -> frontend A

frontend B -> root B -> child B1
                  |
                  +-> execution-scoped service calls -> frontend B
```

A provider name may exist on several frontends. Execution-scoped routing still follows the root owner. A response from another connection is rejected and does not consume the pending request.

Workspace-owned conductor integrations are different. They inspect the live provider catalog, select an eligible connection from advertised capabilities, then address that connection directly. They do not fabricate a root execution merely to reach a frontend service.

The conductor releases a root route when the root becomes terminal. Disconnect releases all routes owned by that connection.

## Requests

The conductor assigns a correlation ID and sends one request to the selected frontend.

```json
{
  "type": "frontend_service_request",
  "id": 31,
  "request": {
    "provider": "web",
    "method": "search",
    "params": {
      "query": "NixOS flakes"
    }
  }
}
```

The frontend returns one success or error response with the same ID.

```json
{
  "id": 31,
  "status": "ok",
  "result": {
    "items": []
  }
}
```

```json
{
  "id": 31,
  "status": "error",
  "error": {
    "code": "provider_failed",
    "message": "search provider failed"
  }
}
```

Requests may be concurrent. Response order does not need to match request order.

If the frontend disconnects while calls are pending, those calls fail. A later connection cannot answer them.

## Notifications

Notifications are one-way and have no correlation ID. They use the same envelope in either direction.

```json
{
  "type": "frontend_service_notification",
  "notification": {
    "provider": "web",
    "method": "invalidate",
    "params": {
      "key": "result-cache"
    }
  }
}
```

For conductor-to-frontend notifications, execution-scoped delivery follows the root owner. Workspace-owned integrations may notify a directly selected live connection.

For frontend-to-conductor notifications, the conductor validates that the sending connection currently advertises the named provider. It then publishes the notification together with its source connection identity to process-local conductor subscribers. Invalid one-way notifications fail the connection input rather than producing a fabricated correlated response.

The generic transport does not persist notifications. A concrete conductor service decides whether consuming a notification produces durable semantic state or an observation.

## Provider descriptors

A descriptor has a stable provider ID and a set of capability strings.

```json
{
  "id": "web",
  "capabilities": ["fetch", "search"]
}
```

Provider IDs route calls. Capabilities let conductor-owned integrations decide whether a live provider satisfies their contract. Capability advertisement does not alter execution authority or callable delegation. Method names and JSON payloads remain provider-specific, so new frontend services do not require a new transport envelope.

The source connection identity is process-local routing state. It is not part of the descriptor sent by the frontend and is never durable.

## Error rules

An execution-scoped service call fails before dispatch when the execution has no live root route or its owning frontend does not advertise the requested provider.

A direct conductor-owned call fails when its selected connection disappeared or no longer advertises the provider.

The conductor rejects duplicate provider IDs in one advertisement. It rejects responses with unknown correlation IDs and responses sent by a connection that does not own the pending call. It rejects inbound notifications naming a provider the sending connection does not advertise.

A remote error is data returned by the frontend service. Transport and lifecycle errors remain conductor-side failures.

## Scope

This slice provides generic bidirectional request, response, notification, registration, capability inspection, direct provider addressing, and execution routing contracts. Concrete services build on it.

LSP integration is outside this slice. The interface does not expose arbitrary frontend IPC to executions. Conductor-owned code chooses when a frontend service is used and which provider method it calls.
