    #[must_use]
    pub fn new(runtime: ConductorRuntime) -> Self {
        Self {
            runtime: Arc::new(Mutex::new(runtime)),
            backends: BTreeMap::new(),
            catalogs: BTreeMap::new(),
            active_scopes: Arc::new(Mutex::new(BTreeMap::new())),
            workspace_leases: WorkspaceLeaseManager::default(),
            workspace_consistency: None,
            store: None,
            persist_lock: Arc::new(Mutex::new(())),
        }
    }

    pub fn load_or_new(store: SqliteStore, workspace_id: WorkspaceId) -> Result<Self, ServerError> {
        let runtime = match store.load() {
            Ok(journal) => {
                let mut runtime = ConductorRuntime::restore(journal)?;
                runtime.bind_workspace(workspace_id.clone())?;
                runtime
            }
            Err(PersistenceError::Io(error)) if error.kind() == io::ErrorKind::NotFound => {
                let mut runtime = ConductorRuntime::new();
                runtime.bind_workspace(workspace_id)?;
                runtime
            }
            Err(error) => return Err(error.into()),
        };
        let mut server = Self::new(runtime);
        server.store = Some(store);
        {
            let mut runtime = server.lock_runtime()?;
            runtime.interrupt_non_resumable_executions()?;
        }
        server.persist()?;
        Ok(server)
    }

    pub fn runtime(&self) -> MutexGuard<'_, ConductorRuntime> {
        self.runtime
            .lock()
            .expect("conductor runtime lock must not be poisoned")
    }
