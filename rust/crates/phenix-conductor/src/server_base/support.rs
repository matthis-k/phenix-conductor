impl ConductorServer {
    fn worker_context(&self) -> ExecutionWorkerContext {
        ExecutionWorkerContext {
            runtime: self.runtime.clone(),
            backends: self.backends.clone(),
            active_scopes: self.active_scopes.clone(),
            workspace_leases: self.workspace_leases.clone(),
            workspace_phases: Arc::new(Mutex::new(BTreeMap::new())),
            workspace_consistency: self.workspace_consistency.clone(),
            store: self.store.clone(),
            persist_lock: self.persist_lock.clone(),
        }
    }
}
