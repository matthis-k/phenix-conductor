CREATE TABLE context_checkpoints (
    recorded_sequence INTEGER PRIMARY KEY REFERENCES domain_events(sequence),
    execution_id TEXT NOT NULL REFERENCES executions(execution_id),
    summary TEXT NOT NULL,
    compactor_target_id INTEGER NOT NULL REFERENCES targets(target_id),
    previous_checkpoint_sequence INTEGER REFERENCES context_checkpoints(recorded_sequence)
);

CREATE TABLE context_checkpoint_ranges (
    checkpoint_sequence INTEGER NOT NULL REFERENCES context_checkpoints(recorded_sequence),
    range_index INTEGER NOT NULL,
    start_sequence INTEGER NOT NULL REFERENCES domain_events(sequence),
    end_sequence INTEGER NOT NULL REFERENCES domain_events(sequence),
    PRIMARY KEY (checkpoint_sequence, range_index),
    CHECK (start_sequence <= end_sequence)
);

CREATE TABLE context_checkpoint_retained_refs (
    checkpoint_sequence INTEGER NOT NULL REFERENCES context_checkpoints(recorded_sequence),
    ref_index INTEGER NOT NULL,
    source_kind TEXT NOT NULL,
    source_id TEXT,
    source_event_sequence INTEGER,
    source_revision TEXT,
    PRIMARY KEY (checkpoint_sequence, ref_index)
);