# Workspace persistence

status: implemented

## Ownership

Each Phenix workspace owns one canonical SQLite database. Git worktrees are distinct workspaces and use distinct databases.

The database is durable conductor state. Process-local frontend, backend, language-server, and terminal resources are not canonical workspace state.

## Location

The default database path is:

```text
$XDG_STATE_HOME/phenix/workspaces/<workspace-key>/workspace.db
```

If `XDG_STATE_HOME` is unset, use the platform XDG-compatible user state fallback. `<workspace-key>` is a filesystem-safe stable digest of the canonical workspace identity.

An explicit state path may override the default for tests or debugging. The override changes storage location, not workspace identity. The repository tree never contains the canonical database.

## Identity

Workspace discovery canonicalizes the workspace root before deriving `WorkspaceId` and the workspace key.

The database records the exact `WorkspaceId` and canonical root. Opening an initialized database for another workspace fails before the conductor serves requests.

Moving or copying a workspace creates a different workspace identity unless a future explicit migration operation changes that identity.

## Startup

Normal conductor startup opens or creates the canonical workspace database. Socket mode does not require a separate persistence flag.

Workspace identity validation and schema migration complete before configuration binding, backend registration, or frontend service begins.

## Migrations

Schema migrations are ordered and transactional. Each migration commits its schema changes and matching `schema_migrations` row in one immediate transaction. The store rejects schema versions newer than the running conductor.

A failed migration prevents workspace startup. The conductor must not expose a runtime backed by a partly migrated database.

Migration code keeps one current relational schema. It does not maintain parallel compatibility models.

## Durable domains

The same database owns runtime history and later workspace knowledge, including objectives, plans, decisions, context records, observations, artifacts, references, and search indexes.

Derived indexes may be rebuilt. Durable semantic rows remain authoritative.

## Deletion

Deletion is dependency-aware. Removing durable material must either remove its dependants coherently or preserve an explicit unavailable-reference state. Silent dangling evidence is invalid.
