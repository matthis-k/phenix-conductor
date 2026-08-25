CREATE TABLE context_resource_revisions (
    recorded_sequence INTEGER PRIMARY KEY,
    resource_id TEXT NOT NULL,
    revision TEXT NOT NULL,
    resource_kind TEXT NOT NULL CHECK (
        resource_kind IN ('skill', 'project_document', 'objective', 'plan')
    ),
    title TEXT NOT NULL,
    description TEXT NOT NULL,
    scope_kind TEXT NOT NULL CHECK (
        scope_kind IN ('workspace', 'execution', 'objective', 'path', 'configuration')
    ),
    scope_id TEXT,
    scope_path TEXT,
    estimated_cost INTEGER NOT NULL CHECK (estimated_cost >= 0),
    tier TEXT NOT NULL CHECK (
        tier IN ('mandatory_content', 'mandatory_metadata', 'discoverable_content')
    ),
    source_kind TEXT NOT NULL,
    source_id TEXT,
    source_event_sequence INTEGER,
    source_revision TEXT NOT NULL,
    content_identity TEXT NOT NULL,
    content TEXT,
    FOREIGN KEY (recorded_sequence) REFERENCES domain_events(sequence),
    UNIQUE (resource_id, revision),
    CHECK (
        (scope_kind = 'path' AND scope_id IS NULL AND scope_path IS NOT NULL)
        OR
        (scope_kind <> 'path' AND scope_id IS NOT NULL AND scope_path IS NULL)
    ),
    CHECK (
        (source_kind = 'event' AND source_id IS NULL AND source_event_sequence IS NOT NULL)
        OR
        (source_kind <> 'event' AND source_id IS NOT NULL AND source_event_sequence IS NULL)
    )
);

CREATE INDEX context_resource_revisions_identity
    ON context_resource_revisions(resource_id, revision);
