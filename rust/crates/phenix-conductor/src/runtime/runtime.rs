impl ConductorRuntime {
    #[must_use]
    pub fn new() -> Self {
        let config_revision = ConfigRevisionId::parse("config-1").expect("static config id");
        let workspace_id = WorkspaceId::parse("workspace:in-memory").expect("static workspace id");
        let configuration = CompiledConfiguration::default();
        let fingerprint = configuration.fingerprint();
        let config_revisions = BTreeMap::from([(
            config_revision.clone(),
            ConfigRevisionSlot {
                fingerprint: fingerprint.clone(),
                configuration: Some(configuration),
                ordinal: 1,
            },
        )]);
        Self {
            journal: RuntimeJournal::new(config_revision.clone(), fingerprint),
            config_revision,
            config_revisions,
            workspace_id,
            sessions: BTreeMap::new(),
            executions: BTreeMap::new(),
            root_ingress: BTreeMap::new(),
            next_root_ingress: BTreeMap::new(),
            attempt_groups: BTreeMap::new(),
            orchestration_decisions: BTreeMap::new(),
            orchestration_interfaces: BTreeMap::new(),
            orchestration_nodes: BTreeMap::new(),
            orchestration_node_inputs: BTreeMap::new(),
            orchestration_synthesis: BTreeMap::new(),
            execution_outputs: BTreeMap::new(),
            diagnostic_write_patches: Vec::new(),
            resolved_routes: BTreeMap::new(),
            read_sets: BTreeMap::new(),
            events: Vec::new(),
            skill_activations: BTreeMap::new(),
            active_lifecycle_hooks: BTreeSet::new(),
            sandbox_states: BTreeMap::new(),
            policy: InvocationPolicy::new(),
            event_sinks: BTreeMap::new(),
            next_event_subscription: 0,
            next_config_revision: 1,
            next_session: 0,
            next_execution: 0,
            next_attempt_group: 0,
            next_event: 0,
            next_tool_call: 0,
        }
    }
}
