CREATE TABLE context_injections (
    recorded_sequence INTEGER PRIMARY KEY,
    execution_id TEXT NOT NULL,
    source_kind TEXT NOT NULL,
    source_id TEXT,
    source_event_sequence INTEGER,
    source_revision TEXT NOT NULL,
    requester TEXT NOT NULL,
    reason TEXT NOT NULL,
    lifetime TEXT NOT NULL,
    content_identity TEXT NOT NULL,
    FOREIGN KEY (recorded_sequence) REFERENCES domain_events(sequence),
    FOREIGN KEY (execution_id) REFERENCES executions(execution_id),
    CHECK (
        (source_kind = 'event' AND source_id IS NULL AND source_event_sequence IS NOT NULL)
        OR
        (source_kind <> 'event' AND source_id IS NOT NULL AND source_event_sequence IS NULL)
    )
);

CREATE INDEX context_injections_execution_sequence
    ON context_injections(execution_id, recorded_sequence);
