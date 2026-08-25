impl ConductorServer {
    fn persist(&self) -> Result<(), ServerError> {
        persist_shared(&self.runtime, self.store.as_ref(), &self.persist_lock)?;
        Ok(())
    }

    fn lock_runtime(&self) -> Result<MutexGuard<'_, ConductorRuntime>, ServerError> {
        self.runtime
            .lock()
            .map_err(|_| ServerError::StatePoisoned("conductor runtime"))
    }
}
