CREATE TABLE objective_creations (
    created_sequence INTEGER PRIMARY KEY REFERENCES domain_events(sequence) ON DELETE CASCADE,
    objective_id TEXT NOT NULL UNIQUE,
    workspace_id TEXT NOT NULL,
    origin_kind TEXT NOT NULL CHECK(origin_kind IN ('root', 'derived')),
    parent_objective_id TEXT REFERENCES objective_creations(objective_id),
    statement TEXT NOT NULL,
    state TEXT NOT NULL,
    supersedes_objective_id TEXT REFERENCES objective_creations(objective_id)
) STRICT;

CREATE TABLE objective_creation_criteria (
    created_sequence INTEGER NOT NULL REFERENCES objective_creations(created_sequence) ON DELETE CASCADE,
    criterion_order INTEGER NOT NULL,
    criterion_id TEXT NOT NULL,
    description TEXT NOT NULL,
    required INTEGER NOT NULL CHECK(required IN (0, 1)),
    PRIMARY KEY(created_sequence, criterion_order),
    UNIQUE(created_sequence, criterion_id)
) STRICT;

CREATE TABLE objective_draft_revisions (
    sequence INTEGER PRIMARY KEY REFERENCES domain_events(sequence) ON DELETE CASCADE,
    objective_id TEXT NOT NULL REFERENCES objective_creations(objective_id),
    statement TEXT NOT NULL
) STRICT;

CREATE TABLE objective_draft_revision_criteria (
    sequence INTEGER NOT NULL REFERENCES objective_draft_revisions(sequence) ON DELETE CASCADE,
    criterion_order INTEGER NOT NULL,
    criterion_id TEXT NOT NULL,
    description TEXT NOT NULL,
    required INTEGER NOT NULL CHECK(required IN (0, 1)),
    PRIMARY KEY(sequence, criterion_order),
    UNIQUE(sequence, criterion_id)
) STRICT;

CREATE TABLE objective_evidence (
    sequence INTEGER PRIMARY KEY REFERENCES domain_events(sequence) ON DELETE CASCADE,
    objective_id TEXT NOT NULL REFERENCES objective_creations(objective_id),
    criterion_id TEXT NOT NULL,
    evidence_ref TEXT NOT NULL
) STRICT;

CREATE TABLE objective_state_changes (
    sequence INTEGER PRIMARY KEY REFERENCES domain_events(sequence) ON DELETE CASCADE,
    objective_id TEXT NOT NULL REFERENCES objective_creations(objective_id),
    from_state TEXT NOT NULL,
    to_state TEXT NOT NULL,
    cause_kind TEXT NOT NULL,
    cause_execution_id TEXT REFERENCES executions(execution_id),
    cause_detail TEXT
) STRICT;

CREATE TABLE execution_objective_assignments (
    sequence INTEGER PRIMARY KEY REFERENCES domain_events(sequence) ON DELETE CASCADE,
    execution_id TEXT NOT NULL UNIQUE REFERENCES executions(execution_id),
    primary_objective_id TEXT NOT NULL REFERENCES objective_creations(objective_id)
) STRICT;

CREATE TABLE execution_supporting_objectives (
    sequence INTEGER NOT NULL REFERENCES execution_objective_assignments(sequence) ON DELETE CASCADE,
    objective_id TEXT NOT NULL REFERENCES objective_creations(objective_id),
    PRIMARY KEY(sequence, objective_id)
) STRICT;
