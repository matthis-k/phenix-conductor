fn persist_shared(
    runtime: &SharedRuntime,
    store: Option<&SqliteStore>,
    persist_lock: &Arc<Mutex<()>>,
) -> Result<(), PersistenceError> {
    let Some(store) = store else {
        return Ok(());
    };
    let _persist_guard = persist_lock
        .lock()
        .map_err(|_| PersistenceError::InvalidJournal("persistence lock poisoned".to_owned()))?;
    let journal = runtime
        .lock()
        .map_err(|_| PersistenceError::InvalidJournal("runtime lock poisoned".to_owned()))?
        .journal()
        .clone();
    store.save(&journal)
}
