CREATE TABLE worker_tasks (
    task_id TEXT PRIMARY KEY,
    parent_execution_id TEXT NOT NULL,
    primary_objective_id TEXT NOT NULL,
    plan_id TEXT,
    plan_step_id TEXT,
    description TEXT NOT NULL,
    profile_id TEXT NOT NULL,
    expected_result_schema_value_id INTEGER NOT NULL,
    delegated_authority_value_id INTEGER NOT NULL,
    created_sequence INTEGER NOT NULL UNIQUE,
    CHECK ((plan_id IS NULL) = (plan_step_id IS NULL))
);

CREATE TABLE worker_task_supporting_objectives (
    task_id TEXT NOT NULL,
    ordinal INTEGER NOT NULL,
    objective_id TEXT NOT NULL,
    PRIMARY KEY (task_id, ordinal),
    UNIQUE (task_id, objective_id),
    FOREIGN KEY (task_id) REFERENCES worker_tasks(task_id)
);

CREATE TABLE worker_task_dependencies (
    task_id TEXT NOT NULL,
    ordinal INTEGER NOT NULL,
    dependency_task_id TEXT NOT NULL,
    PRIMARY KEY (task_id, ordinal),
    UNIQUE (task_id, dependency_task_id),
    FOREIGN KEY (task_id) REFERENCES worker_tasks(task_id)
);

CREATE TABLE worker_task_input_refs (
    task_id TEXT NOT NULL,
    ordinal INTEGER NOT NULL,
    reference_value_id INTEGER NOT NULL,
    PRIMARY KEY (task_id, ordinal),
    FOREIGN KEY (task_id) REFERENCES worker_tasks(task_id)
);

CREATE TABLE worker_task_state_events (
    sequence INTEGER PRIMARY KEY,
    task_id TEXT NOT NULL,
    event_kind TEXT NOT NULL,
    execution_id TEXT NOT NULL,
    cause TEXT,
    FOREIGN KEY (task_id) REFERENCES worker_tasks(task_id)
);

CREATE TABLE worker_task_result_refs (
    sequence INTEGER NOT NULL,
    ordinal INTEGER NOT NULL,
    reference_value_id INTEGER NOT NULL,
    PRIMARY KEY (sequence, ordinal),
    FOREIGN KEY (sequence) REFERENCES worker_task_state_events(sequence)
);
