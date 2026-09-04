use crate::{
    Authority, ComponentManifest, GraphReconciler, Kernel, LiveReconciliationError,
    PluginExecution, PluginId, PluginManifest, ReconciliationResult, ResolvedHarness,
    ResolvedHarnessError,
};
use serde::{Deserialize, Serialize};
use std::{
    error::Error,
    fmt::{self, Display, Formatter},
};

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "operation", content = "request", rename_all = "snake_case")]
pub enum PluginManagementRequest {
    Load(Box<PluginLoadRequest>),
    Unload(PluginUnloadRequest),
    Reconcile(PluginSetRequest),
}

impl PluginManagementRequest {
    #[must_use]
    pub fn load(request: PluginLoadRequest) -> Self {
        Self::Load(Box::new(request))
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct PluginLoadRequest {
    pub manifest: PluginManifest,
    pub components: Vec<ComponentManifest>,
    pub expected_active_revision: Option<String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct PluginUnloadRequest {
    pub plugin: PluginId,
    pub expected_active_revision: Option<String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct PluginSetRequest {
    pub plugins: Vec<PluginManifest>,
    pub components: Vec<ComponentManifest>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum PluginManagementError {
    UnknownPlugin(PluginId),
    StaleExpectedRevision {
        plugin: PluginId,
        expected: String,
        active: Option<String>,
    },
    Candidate(ResolvedHarnessError),
    Reconciliation(LiveReconciliationError),
}

impl Display for PluginManagementError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownPlugin(plugin) => write!(f, "unknown plugin: {plugin}"),
            Self::StaleExpectedRevision {
                plugin,
                expected,
                active,
            } => {
                let active = active.as_deref().unwrap_or("<none>");
                write!(
                    f,
                    "plugin {plugin} active revision {active} does not match expected {expected}"
                )
            }
            Self::Candidate(error) => write!(
                f,
                "plugin management candidate resolution failed: {error:?}"
            ),
            Self::Reconciliation(error) => {
                write!(f, "plugin management reconciliation failed: {error:?}")
            }
        }
    }
}

impl Error for PluginManagementError {}

impl From<ResolvedHarnessError> for PluginManagementError {
    fn from(error: ResolvedHarnessError) -> Self {
        Self::Candidate(error)
    }
}

impl From<LiveReconciliationError> for PluginManagementError {
    fn from(error: LiveReconciliationError) -> Self {
        Self::Reconciliation(error)
    }
}

impl GraphReconciler {
    /// Apply one kernel-owned desired-state plugin management request.
    ///
    /// Load and unload are convenience mutations of the active desired plugin
    /// set; reconcile supplies the complete desired set. Each request resolves
    /// one candidate generation before the live kernel commits it atomically.
    /// Loading an already-active `PluginId` with a different manifest is a
    /// replacement; there is no separate reload path.
    pub fn manage(
        &mut self,
        kernel: &mut Kernel,
        request: PluginManagementRequest,
        authority_ceiling: &Authority,
    ) -> Result<ReconciliationResult, PluginManagementError> {
        let (plugins, components) = match request {
            PluginManagementRequest::Load(request) => apply_load(self.active(), *request)?,
            PluginManagementRequest::Unload(request) => apply_unload(self.active(), request)?,
            PluginManagementRequest::Reconcile(request) => apply_reconcile(request),
        };
        let candidate = self
            .active()
            .with_plugin_set(plugins, components, authority_ceiling)?;
        self.activate_candidate_on_kernel(kernel, candidate)
            .map_err(PluginManagementError::from)
    }
}

fn apply_load(
    active: &ResolvedHarness,
    request: PluginLoadRequest,
) -> Result<(Vec<PluginManifest>, Vec<ComponentManifest>), PluginManagementError> {
    let plugin = request.manifest.id.clone();
    let existing = active
        .plugins()
        .iter()
        .find(|manifest| manifest.id == plugin);
    if let Some(expected) = &request.expected_active_revision {
        check_expected_revision(&plugin, existing, expected)?;
    }
    let mut plugins: Vec<_> = active.plugins().to_vec();
    match plugins.iter_mut().find(|manifest| manifest.id == plugin) {
        Some(slot) => *slot = request.manifest,
        None => plugins.push(request.manifest),
    }
    let mut components: Vec<_> = active
        .components()
        .iter()
        .filter(|component| component.owner != plugin)
        .cloned()
        .collect();
    components.extend(request.components);
    Ok((plugins, components))
}

fn apply_unload(
    active: &ResolvedHarness,
    request: PluginUnloadRequest,
) -> Result<(Vec<PluginManifest>, Vec<ComponentManifest>), PluginManagementError> {
    let existing = active
        .plugins()
        .iter()
        .find(|manifest| manifest.id == request.plugin)
        .ok_or_else(|| PluginManagementError::UnknownPlugin(request.plugin.clone()))?;
    if let Some(expected) = &request.expected_active_revision {
        check_expected_revision(&request.plugin, Some(existing), expected)?;
    }
    let plugins = active
        .plugins()
        .iter()
        .filter(|manifest| manifest.id != request.plugin)
        .cloned()
        .collect();
    let components = active
        .components()
        .iter()
        .filter(|component| component.owner != request.plugin)
        .cloned()
        .collect();
    Ok((plugins, components))
}

fn apply_reconcile(request: PluginSetRequest) -> (Vec<PluginManifest>, Vec<ComponentManifest>) {
    (request.plugins, request.components)
}

fn check_expected_revision(
    plugin: &PluginId,
    active: Option<&PluginManifest>,
    expected: &str,
) -> Result<(), PluginManagementError> {
    let revision = active.and_then(|manifest| match &manifest.execution {
        PluginExecution::Runtime { artifact, .. } => Some(artifact.revision.clone()),
        PluginExecution::Embedded | PluginExecution::ResourceOnly => None,
    });
    if revision.as_deref() == Some(expected) {
        return Ok(());
    }
    Err(PluginManagementError::StaleExpectedRevision {
        plugin: plugin.clone(),
        expected: expected.to_owned(),
        active: revision,
    })
}
