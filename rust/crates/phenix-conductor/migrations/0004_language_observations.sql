CREATE TABLE language_observations (
    recorded_sequence INTEGER PRIMARY KEY REFERENCES domain_events(sequence),
    execution_id TEXT NOT NULL REFERENCES executions(execution_id),
    workspace_id TEXT NOT NULL,
    service_kind TEXT NOT NULL,
    provider_id TEXT NOT NULL,
    provider_epoch INTEGER NOT NULL CHECK (provider_epoch >= 0),
    operation_value_id INTEGER NOT NULL REFERENCES structured_values(value_id),
    result_value_id INTEGER NOT NULL REFERENCES structured_values(value_id)
) STRICT;

CREATE INDEX language_observations_by_execution
ON language_observations(execution_id, recorded_sequence);
