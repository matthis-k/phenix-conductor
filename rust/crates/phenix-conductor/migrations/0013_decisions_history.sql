CREATE TABLE context_resource_revisions_v13 (
    recorded_sequence INTEGER PRIMARY KEY,
    resource_id TEXT NOT NULL,
    revision TEXT NOT NULL,
    resource_kind TEXT NOT NULL CHECK (
        resource_kind IN ('skill', 'project_document', 'objective', 'plan', 'decision', 'artifact')
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

INSERT INTO context_resource_revisions_v13 (
    recorded_sequence, resource_id, revision, resource_kind, title, description,
    scope_kind, scope_id, scope_path, estimated_cost, tier, source_kind, source_id,
    source_event_sequence, source_revision, content_identity, content
)
SELECT
    recorded_sequence, resource_id, revision, resource_kind, title, description,
    scope_kind, scope_id, scope_path, estimated_cost, tier, source_kind, source_id,
    source_event_sequence, source_revision, content_identity, content
FROM context_resource_revisions;

DROP INDEX context_resource_revisions_identity;
DROP TABLE context_resource_revisions;
ALTER TABLE context_resource_revisions_v13 RENAME TO context_resource_revisions;
CREATE INDEX context_resource_revisions_identity
    ON context_resource_revisions(resource_id, revision);

CREATE TABLE decision_snapshots (
    sequence INTEGER PRIMARY KEY REFERENCES domain_events(sequence) ON DELETE CASCADE,
    decision_id TEXT NOT NULL,
    workspace_id TEXT NOT NULL,
    revision INTEGER NOT NULL,
    question TEXT NOT NULL,
    chosen_option TEXT NOT NULL,
    rationale TEXT NOT NULL,
    creator_kind TEXT NOT NULL CHECK(creator_kind IN ('user', 'execution')),
    creator_execution_id TEXT REFERENCES executions(execution_id),
    relation_kind TEXT CHECK(relation_kind IN ('supersedes', 'reverts')),
    relation_decision_id TEXT,
    UNIQUE(decision_id, revision)
) STRICT;

CREATE TABLE decision_alternatives (
    sequence INTEGER NOT NULL REFERENCES decision_snapshots(sequence) ON DELETE CASCADE,
    alternative_order INTEGER NOT NULL,
    alternative TEXT NOT NULL,
    PRIMARY KEY(sequence, alternative_order)
) STRICT;

CREATE TABLE decision_no_alternative_reasons (
    sequence INTEGER PRIMARY KEY REFERENCES decision_snapshots(sequence) ON DELETE CASCADE,
    reason TEXT NOT NULL CHECK(length(trim(reason)) > 0)
) STRICT;

CREATE TABLE decision_evidence_refs (
    sequence INTEGER NOT NULL REFERENCES decision_snapshots(sequence) ON DELETE CASCADE,
    evidence_order INTEGER NOT NULL,
    source_kind TEXT NOT NULL,
    source_id TEXT,
    source_event_sequence INTEGER,
    source_revision TEXT,
    PRIMARY KEY(sequence, evidence_order)
) STRICT;

CREATE TABLE decision_objectives (
    sequence INTEGER NOT NULL REFERENCES decision_snapshots(sequence) ON DELETE CASCADE,
    objective_id TEXT NOT NULL REFERENCES objective_creations(objective_id),
    PRIMARY KEY(sequence, objective_id)
) STRICT;

CREATE TABLE decision_dependencies (
    sequence INTEGER NOT NULL REFERENCES decision_snapshots(sequence) ON DELETE CASCADE,
    dependency_decision_id TEXT NOT NULL,
    PRIMARY KEY(sequence, dependency_decision_id)
) STRICT;

CREATE TABLE decision_recordings (
    sequence INTEGER PRIMARY KEY REFERENCES domain_events(sequence) ON DELETE CASCADE,
    decision_id TEXT NOT NULL UNIQUE
) STRICT;

CREATE TABLE decision_applicability_assessments (
    sequence INTEGER PRIMARY KEY REFERENCES domain_events(sequence) ON DELETE CASCADE,
    decision_id TEXT NOT NULL,
    applicability TEXT NOT NULL CHECK(applicability IN ('applicable', 'questionable', 'invalidated'))
) STRICT;

CREATE VIRTUAL TABLE decision_fts USING fts5(
    decision_id UNINDEXED,
    question,
    chosen_option,
    rationale,
    alternatives
);
