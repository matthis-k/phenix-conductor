CREATE TABLE execution_worker_profiles (
    execution_id TEXT PRIMARY KEY REFERENCES executions(execution_id),
    profile_id TEXT NOT NULL,
    bound_sequence INTEGER NOT NULL UNIQUE REFERENCES domain_events(sequence)
) STRICT;
