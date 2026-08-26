CREATE TABLE terminal_resources (
    terminal_id TEXT PRIMARY KEY,
    owner_kind TEXT NOT NULL,
    owner_id TEXT NOT NULL,
    created_by_execution_id TEXT NOT NULL,
    authority_value_id INTEGER NOT NULL,
    created_sequence INTEGER NOT NULL UNIQUE
);

CREATE TABLE job_resources (
    job_id TEXT PRIMARY KEY,
    owner_kind TEXT NOT NULL,
    owner_id TEXT NOT NULL,
    created_by_execution_id TEXT NOT NULL,
    authority_value_id INTEGER NOT NULL,
    created_sequence INTEGER NOT NULL UNIQUE
);

CREATE TABLE process_resource_events (
    sequence INTEGER PRIMARY KEY,
    resource_kind TEXT NOT NULL,
    resource_id TEXT NOT NULL,
    event_kind TEXT NOT NULL,
    owner_id TEXT,
    exit_code INTEGER,
    output_ref_value_id INTEGER
);
