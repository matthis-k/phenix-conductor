CREATE TABLE context_resource_revisions_v10 (
    recorded_sequence INTEGER PRIMARY KEY,
    resource_id TEXT NOT NULL,
    revision TEXT NOT NULL,
    resource_kind TEXT NOT NULL CHECK (
        resource_kind IN ('skill', 'project_document', 'objective', 'plan', 'artifact')
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

INSERT INTO context_resource_revisions_v10 (
    recorded_sequence,
    resource_id,
    revision,
    resource_kind,
    title,
    description,
    scope_kind,
    scope_id,
    scope_path,
    estimated_cost,
    tier,
    source_kind,
    source_id,
    source_event_sequence,
    source_revision,
    content_identity,
    content
)
SELECT
    recorded_sequence,
    resource_id,
    revision,
    resource_kind,
    title,
    description,
    scope_kind,
    scope_id,
    scope_path,
    estimated_cost,
    tier,
    source_kind,
    source_id,
    source_event_sequence,
    source_revision,
    content_identity,
    content
FROM context_resource_revisions;

DROP INDEX context_resource_revisions_identity;
DROP TABLE context_resource_revisions;
ALTER TABLE context_resource_revisions_v10 RENAME TO context_resource_revisions;

CREATE INDEX context_resource_revisions_identity
    ON context_resource_revisions(resource_id, revision);
