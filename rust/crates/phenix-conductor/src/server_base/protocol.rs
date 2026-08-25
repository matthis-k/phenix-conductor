    pub fn serve_ndjson<R, W>(&mut self, input: R, output: W) -> Result<(), ServerError>
    where
        R: BufRead,
        W: Write + Send,
    {
        let mut on_root = |_: &ExecutionId| Ok(());
        self.serve_ndjson_with_root_hook(input, output, &mut on_root)
    }

    pub(super) fn serve_ndjson_with_root_hook<R, W>(
        &mut self,
        input: R,
        output: W,
        on_root: RootAcceptedHook<'_>,
    ) -> Result<(), ServerError>
    where
        R: BufRead,
        W: Write + Send,
    {
        let (event_subscription, event_receiver) = {
            let mut runtime = self.lock_runtime()?;
            runtime.subscribe_events_with_id(EVENT_BUFFER)
        };
        let (output_sender, output_receiver) = mpsc::sync_channel(OUTPUT_BUFFER);
        let executions = ExecutionQueue::default();
        let worker_context = ExecutionWorkerContext {
            runtime: self.runtime.clone(),
            backends: self.backends.clone(),
            active_scopes: self.active_scopes.clone(),
            workspace_leases: self.workspace_leases.clone(),
            workspace_phases: Arc::new(Mutex::new(BTreeMap::new())),
            workspace_consistency: self.workspace_consistency.clone(),
            store: self.store.clone(),
            persist_lock: self.persist_lock.clone(),
        };

        thread::scope(|scope| {
            let writer = scope.spawn(move || -> Result<(), ServerError> {
                let mut output = output;
                while let Ok(message) = output_receiver.recv() {
                    serde_json::to_writer(&mut output, &message)?;
                    output.write_all(b"\n")?;
                    output.flush()?;
                }
                Ok(())
            });

            let event_output = output_sender.clone();
            let relay = scope.spawn(move || {
                while let Ok(event) = event_receiver.recv() {
                    if event_output.send(ServerMessage::Event { event }).is_err() {
                        break;
                    }
                }
            });

            let executors = (0..EXECUTION_WORKERS)
                .map(|_| {
                    let executions = executions.clone();
                    let context = worker_context.clone();
                    scope.spawn(move || execution_loop(executions, context))
                })
                .collect::<Vec<_>>();

            let result = self.read_requests(input, &output_sender, &executions, on_root);
            executions.close()?;
            let mut executor_result = Ok(());
            for executor in executors {
                let worker_result = executor.join().map_err(|_| ServerError::WorkerPanicked)?;
                if executor_result.is_ok() {
                    executor_result = worker_result;
                }
            }

            {
                let mut runtime = self.lock_runtime()?;
                runtime.unsubscribe_event_subscription(event_subscription);
            }
            drop(output_sender);

            relay.join().map_err(|_| ServerError::WorkerPanicked)?;
            let writer_result = writer.join().map_err(|_| ServerError::WorkerPanicked)?;
            result.and(executor_result).and(writer_result)
        })
    }

    fn read_requests<R: BufRead>(
        &mut self,
        input: R,
        output: &SyncSender<ServerMessage>,
        executions: &ExecutionQueue,
        on_root: RootAcceptedHook<'_>,
    ) -> Result<(), ServerError> {
        for line in input.lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            match serde_json::from_str::<ClientMessage>(&line) {
                Ok(message) => self.handle_message(message, output, executions, on_root)?,
                Err(error) => self.respond(
                    output,
                    0,
                    Err(protocol_error(
                        ErrorCode::InvalidRequest,
                        format!("invalid client message: {error}"),
                    )),
                )?,
            }
        }
        Ok(())
    }

    fn handle_message(
        &mut self,
        message: ClientMessage,
        output: &SyncSender<ServerMessage>,
        executions: &ExecutionQueue,
        on_root: RootAcceptedHook<'_>,
    ) -> Result<(), ServerError> {
        let id = message.id;
        match &message.command {
            Command::Submit { session_id, text } => {
                return self.submit(
                    id,
                    session_id.clone(),
                    text.clone(),
                    output,
                    executions,
                    on_root,
                );
            }
            Command::StartCallable {
                session_id,
                callable,
                input,
            } => {
                return self.start_callable(
                    id,
                    session_id.clone(),
                    callable.clone(),
                    input.clone(),
                    output,
                    executions,
                    on_root,
                );
            }
            Command::GetCallableCatalog => {
                let callables = self.lock_runtime()?.callable_descriptors()?;
                self.respond(output, id, Ok(Reply::CallableCatalog { callables }))?;
                return Ok(());
            }
            Command::GetRoutingCatalog => {
                let profiles = self.lock_runtime()?.routing_profiles()?;
                self.respond(output, id, Ok(Reply::RoutingCatalog { profiles }))?;
                return Ok(());
            }
            Command::GetSkillCatalog => {
                let skills = self.lock_runtime()?.skill_descriptors()?;
                self.respond(output, id, Ok(Reply::SkillCatalog { skills }))?;
                return Ok(());
            }
            Command::ExportSessionDebug { session_id } => {
                let reply = self.export_session_debug(session_id);
                self.respond(output, id, reply)?;
                return Ok(());
            }
            _ => {}
        }
        let persist = matches!(
            &message.command,
            Command::CreateSession { .. }
                | Command::ForkSession { .. }
                | Command::RenameSession { .. }
                | Command::SetSessionTarget { .. }
                | Command::RebaseSession { .. }
                | Command::CloseSession { .. }
                | Command::RequestWorkspaceCheckpoint { .. }
                | Command::CancelExecution { .. }
        );

        let reply = match message.command {
            Command::Initialize { after_sequence } => self
                .refresh_all_catalogs()
                .map_err(map_backend_error)
                .and_then(|()| {
                    let runtime = self.lock_runtime().map_err(|error| {
                        protocol_error(ErrorCode::BackendProtocol, error.to_string())
                    })?;
                    Ok(Reply::Initialized {
                        snapshot: runtime.snapshot(),
                        events: runtime.events_since(after_sequence.unwrap_or(0)),
                        backends: self.catalogs(),
                    })
                }),
            Command::GetSnapshot => {
                let runtime = self.lock_runtime()?;
                Ok(Reply::Snapshot {
                    snapshot: runtime.snapshot(),
                    backends: self.catalogs(),
                })
            }
            Command::CreateSession {
                parent_session,
                name,
                target,
            } => self
                .lock_runtime()?
                .create_session(parent_session, name, target)
                .map(|session| Reply::Session { session })
                .map_err(map_conductor_error),
            Command::ForkSession { session_id, name } => self
                .lock_runtime()?
                .fork_session(&session_id, name)
                .map(|session| Reply::Session { session })
                .map_err(map_conductor_error),
            Command::RenameSession { session_id, name } => self
                .lock_runtime()?
                .rename_session(&session_id, name)
                .map(|session| Reply::Session { session })
                .map_err(map_conductor_error),
            Command::SetSessionTarget { session_id, target } => self
                .lock_runtime()?
                .set_session_target(&session_id, target)
                .map(|session| Reply::Session { session })
                .map_err(map_conductor_error),
            Command::RebaseSession {
                session_id,
                config_revision,
            } => self
                .lock_runtime()?
                .rebase_session(&session_id, &config_revision)
                .map(|session| Reply::Session { session })
                .map_err(map_conductor_error),
            Command::CloseSession { session_id } => self.close_session(&session_id),
            Command::CancelExecution { execution_id } => self.cancel_execution(&execution_id),
            Command::RequestWorkspaceCheckpoint { execution_id } => {
                self.capture_workspace_checkpoint(&execution_id)
            }
            Command::RefreshBackendCatalog { backend_id } => self
                .refresh_backend(&backend_id)
                .map(|catalog| Reply::BackendCatalog { catalog })
                .map_err(map_backend_error),
            Command::SelectAuthentication {
                backend_id,
                method_id,
                input,
            } => self
                .authenticate(&backend_id, &method_id, input.as_ref())
                .map(|catalog| Reply::BackendCatalog { catalog })
                .map_err(map_backend_error),
            Command::Submit { .. } => unreachable!("submit handled before dispatch"),
            Command::StartCallable { .. } => {
                unreachable!("callable start handled before dispatch")
            }
            Command::GetCallableCatalog => {
                unreachable!("callable catalog handled before dispatch")
            }
            Command::GetRoutingCatalog => {
                unreachable!("routing catalog handled before dispatch")
            }
            Command::GetSkillCatalog => {
                unreachable!("skill catalog handled before dispatch")
            }
            Command::ExportSessionDebug { .. } => {
                unreachable!("debug export handled before dispatch")
            }
        };

        if persist {
            self.persist()?;
        }
        self.respond(output, id, reply)?;
        Ok(())
    }

    fn respond(
        &self,
        output: &SyncSender<ServerMessage>,
        id: u64,
        result: Result<Reply, ProtocolError>,
    ) -> Result<(), ServerError> {
        let response = match result {
            Ok(result) => ResponsePayload::Ok { result },
            Err(error) => ResponsePayload::Error { error },
        };
        output
            .send(ServerMessage::Response { id, response })
            .map_err(|_| ServerError::OutputClosed)
    }
