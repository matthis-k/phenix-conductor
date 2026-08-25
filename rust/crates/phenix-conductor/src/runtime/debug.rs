impl ConductorRuntime {
    #[must_use]
    pub fn snapshot(&self) -> RuntimeSnapshot {
        RuntimeSnapshot {
            sessions: self
                .sessions
                .values()
                .map(|record| record.summary.clone())
                .collect(),
            executions: self
                .executions
                .values()
                .map(|record| record.summary.clone())
                .collect(),
            last_event_sequence: self.next_event,
        }
    }
}
