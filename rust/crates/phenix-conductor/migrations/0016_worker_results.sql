CREATE TABLE worker_task_verification_requirements (
    sequence INTEGER PRIMARY KEY,
    task_id TEXT NOT NULL UNIQUE,
    FOREIGN KEY (task_id) REFERENCES worker_tasks(task_id)
);

CREATE TABLE worker_results (
    sequence INTEGER PRIMARY KEY,
    task_id TEXT NOT NULL UNIQUE,
    execution_id TEXT NOT NULL,
    output_value_id INTEGER NOT NULL,
    FOREIGN KEY (task_id) REFERENCES worker_tasks(task_id)
);

CREATE TABLE worker_result_evidence_refs (
    sequence INTEGER NOT NULL,
    ordinal INTEGER NOT NULL,
    reference_value_id INTEGER NOT NULL,
    PRIMARY KEY (sequence, ordinal),
    FOREIGN KEY (sequence) REFERENCES worker_results(sequence)
);

CREATE TABLE worker_result_artifact_refs (
    sequence INTEGER NOT NULL,
    ordinal INTEGER NOT NULL,
    reference_value_id INTEGER NOT NULL,
    PRIMARY KEY (sequence, ordinal),
    FOREIGN KEY (sequence) REFERENCES worker_results(sequence)
);

CREATE TABLE worker_verifications (
    sequence INTEGER PRIMARY KEY,
    task_id TEXT NOT NULL,
    verifier_execution_id TEXT NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('passed', 'failed')),
    reason TEXT,
    FOREIGN KEY (task_id) REFERENCES worker_tasks(task_id),
    CHECK ((status = 'passed' AND reason IS NULL) OR (status = 'failed' AND reason IS NOT NULL))
);

CREATE TABLE worker_verification_evidence_refs (
    sequence INTEGER NOT NULL,
    ordinal INTEGER NOT NULL,
    reference_value_id INTEGER NOT NULL,
    PRIMARY KEY (sequence, ordinal),
    FOREIGN KEY (sequence) REFERENCES worker_verifications(sequence)
);

CREATE TABLE worker_failure_analyses (
    sequence INTEGER PRIMARY KEY,
    task_id TEXT NOT NULL,
    analyzer_execution_id TEXT NOT NULL,
    diagnosis TEXT NOT NULL,
    proposed_action TEXT NOT NULL,
    FOREIGN KEY (task_id) REFERENCES worker_tasks(task_id)
);

CREATE TABLE worker_failure_analysis_evidence_refs (
    sequence INTEGER NOT NULL,
    ordinal INTEGER NOT NULL,
    reference_value_id INTEGER NOT NULL,
    PRIMARY KEY (sequence, ordinal),
    FOREIGN KEY (sequence) REFERENCES worker_failure_analyses(sequence)
);
