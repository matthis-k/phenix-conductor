    pub fn register_backend(
        &mut self,
        backend_id: BackendId,
        backend: Box<dyn Backend>,
    ) -> Result<(), ServerError> {
        if self.backends.contains_key(&backend_id) {
            return Err(ServerError::DuplicateBackend(backend_id));
        }
        self.backends
            .insert(backend_id, Arc::new(Mutex::new(backend)));
        Ok(())
    }

    #[must_use]
    pub fn catalogs(&self) -> Vec<BackendCatalog> {
        self.catalogs.values().cloned().collect()
    }

    fn refresh_all_catalogs(&mut self) -> Result<(), BackendError> {
        let backend_ids = self.backends.keys().cloned().collect::<Vec<_>>();
        for backend_id in backend_ids {
            self.refresh_backend(&backend_id)?;
        }
        Ok(())
    }

    fn refresh_backend(&mut self, backend_id: &BackendId) -> Result<BackendCatalog, BackendError> {
        let backend = self.backends.get(backend_id).ok_or_else(|| {
            BackendError::Unsupported(format!("backend is not registered: {backend_id}"))
        })?;
        let catalog = backend
            .lock()
            .map_err(|_| BackendError::Transport("backend lock poisoned".to_owned()))?
            .catalog()?;
        if catalog.backend != *backend_id {
            return Err(BackendError::Protocol(format!(
                "backend catalog id {} does not match registry key {backend_id}",
                catalog.backend
            )));
        }
        self.catalogs.insert(backend_id.clone(), catalog.clone());
        Ok(catalog)
    }

    fn authenticate(
        &mut self,
        backend_id: &BackendId,
        method_id: &AuthenticationMethodId,
        input: Option<&AuthenticationInput>,
    ) -> Result<BackendCatalog, BackendError> {
        let backend = self.backends.get(backend_id).ok_or_else(|| {
            BackendError::Unsupported(format!("backend is not registered: {backend_id}"))
        })?;
        backend
            .lock()
            .map_err(|_| BackendError::Transport("backend lock poisoned".to_owned()))?
            .authenticate_with_input(method_id, input)?;
        self.refresh_backend(backend_id)
    }
