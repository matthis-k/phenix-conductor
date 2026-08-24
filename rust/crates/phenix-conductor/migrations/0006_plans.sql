CREATE TABLE plan_creations (
    sequence INTEGER PRIMARY KEY REFERENCES domain_events(sequence),
    plan_id TEXT NOT NULL UNIQUE,
    workspace_id TEXT NOT NULL,
    revision INTEGER NOT NULL CHECK (revision = 1),
    supersedes_plan_id TEXT REFERENCES plan_creations(plan_id)
) STRICT;

CREATE TABLE plan_creation_objectives (
    sequence INTEGER NOT NULL REFERENCES plan_creations(sequence) ON DELETE CASCADE,
    objective_id TEXT NOT NULL,
    PRIMARY KEY(sequence, objective_id)
) STRICT;

CREATE TABLE plan_creation_steps (
    sequence INTEGER NOT NULL REFERENCES plan_creations(sequence) ON DELETE CASCADE,
    step_order INTEGER NOT NULL,
    step_id TEXT NOT NULL,
    description TEXT NOT NULL,
    state TEXT NOT NULL,
    revisability TEXT NOT NULL,
    PRIMARY KEY(sequence, step_id),
    UNIQUE(sequence, step_order)
) STRICT;

CREATE TABLE plan_creation_step_dependencies (
    sequence INTEGER NOT NULL,
    step_id TEXT NOT NULL,
    dependency_step_id TEXT NOT NULL,
    PRIMARY KEY(sequence, step_id, dependency_step_id),
    FOREIGN KEY(sequence, step_id)
        REFERENCES plan_creation_steps(sequence, step_id) ON DELETE CASCADE
) STRICT;

CREATE TABLE plan_creation_step_objectives (
    sequence INTEGER NOT NULL,
    step_id TEXT NOT NULL,
    objective_id TEXT NOT NULL,
    PRIMARY KEY(sequence, step_id, objective_id),
    FOREIGN KEY(sequence, step_id)
        REFERENCES plan_creation_steps(sequence, step_id) ON DELETE CASCADE
) STRICT;

CREATE TABLE plan_draft_revisions (
    sequence INTEGER PRIMARY KEY REFERENCES domain_events(sequence),
    plan_id TEXT NOT NULL REFERENCES plan_creations(plan_id),
    expected_revision INTEGER NOT NULL,
    revision INTEGER NOT NULL
) STRICT;

CREATE TABLE plan_revision_objectives (
    sequence INTEGER NOT NULL REFERENCES plan_draft_revisions(sequence) ON DELETE CASCADE,
    objective_id TEXT NOT NULL,
    PRIMARY KEY(sequence, objective_id)
) STRICT;

CREATE TABLE plan_revision_steps (
    sequence INTEGER NOT NULL REFERENCES plan_draft_revisions(sequence) ON DELETE CASCADE,
    step_order INTEGER NOT NULL,
    step_id TEXT NOT NULL,
    description TEXT NOT NULL,
    state TEXT NOT NULL,
    revisability TEXT NOT NULL,
    PRIMARY KEY(sequence, step_id),
    UNIQUE(sequence, step_order)
) STRICT;

CREATE TABLE plan_revision_step_dependencies (
    sequence INTEGER NOT NULL,
    step_id TEXT NOT NULL,
    dependency_step_id TEXT NOT NULL,
    PRIMARY KEY(sequence, step_id, dependency_step_id),
    FOREIGN KEY(sequence, step_id)
        REFERENCES plan_revision_steps(sequence, step_id) ON DELETE CASCADE
) STRICT;

CREATE TABLE plan_revision_step_objectives (
    sequence INTEGER NOT NULL,
    step_id TEXT NOT NULL,
    objective_id TEXT NOT NULL,
    PRIMARY KEY(sequence, step_id, objective_id),
    FOREIGN KEY(sequence, step_id)
        REFERENCES plan_revision_steps(sequence, step_id) ON DELETE CASCADE
) STRICT;

CREATE TABLE plan_state_changes (
    sequence INTEGER PRIMARY KEY REFERENCES domain_events(sequence),
    plan_id TEXT NOT NULL REFERENCES plan_creations(plan_id),
    from_state TEXT NOT NULL,
    to_state TEXT NOT NULL,
    cause_kind TEXT NOT NULL,
    cause_execution_id TEXT,
    cause_detail TEXT
) STRICT;

CREATE TABLE plan_step_state_changes (
    sequence INTEGER PRIMARY KEY REFERENCES domain_events(sequence),
    plan_id TEXT NOT NULL REFERENCES plan_creations(plan_id),
    step_id TEXT NOT NULL,
    from_state TEXT NOT NULL,
    to_state TEXT NOT NULL,
    cause_kind TEXT NOT NULL,
    cause_execution_id TEXT,
    cause_detail TEXT
) STRICT;

CREATE TABLE execution_plan_assignments (
    sequence INTEGER PRIMARY KEY REFERENCES domain_events(sequence),
    execution_id TEXT NOT NULL UNIQUE REFERENCES executions(execution_id),
    plan_id TEXT NOT NULL REFERENCES plan_creations(plan_id),
    step_id TEXT NOT NULL
) STRICT;
