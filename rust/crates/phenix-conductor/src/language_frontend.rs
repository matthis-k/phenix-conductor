use super::frontend_services::{
    FrontendConnectionId, FrontendServiceInboundNotification, FrontendServiceRouter,
    FrontendServiceRouterError,
};
use phenix_core::{
    ActiveLanguageProvider, LanguageProviderCandidate, LanguageProviderCapabilities,
    LanguageProviderId, LanguageProviderSource, LanguageServiceConfiguration, LanguageServiceError,
    LanguageServiceKind, LanguageServiceManager, WorkspaceId,
};
use phenix_protocol::{
    FrontendServiceProviderDescriptor, FrontendServiceProviderId, FrontendServiceRequest,
};
use serde_json::Value;
use std::collections::BTreeMap;
use std::fmt::{self, Display, Formatter};
use std::sync::{Arc, Mutex};

const PROVIDER_PREFIX: &str = "language/";
const REQUESTS: &str = "language.requests";
const NOTIFICATIONS: &str = "language.notifications";
const SHARED_DIAGNOSTICS: &str = "language.shared_diagnostics";
const BACKGROUND_DOCUMENTS: &str = "language.background_documents";
const DIRTY_BUFFERS: &str = "language.dirty_buffers";

#[derive(Clone, Debug)]
struct FrontendLanguageCandidate {
    connection: FrontendConnectionId,
    frontend_provider: FrontendServiceProviderId,
    candidate: LanguageProviderCandidate,
}

#[derive(Clone, Debug)]
struct FrontendLanguageBinding {
    connection: FrontendConnectionId,
    frontend_provider: FrontendServiceProviderId,
    epoch: u64,
}

#[derive(Default)]
struct FrontendLanguageState {
    manager: LanguageServiceManager,
    connection_aliases: BTreeMap<FrontendConnectionId, u64>,
    next_connection_alias: u64,
    bindings: BTreeMap<(WorkspaceId, LanguageServiceKind), FrontendLanguageBinding>,
}

#[derive(Clone, Default)]
pub(super) struct FrontendLanguageServices {
    state: Arc<Mutex<FrontendLanguageState>>,
}

#[derive(Clone, Debug, PartialEq)]
pub(super) enum FrontendLanguageServiceError {
    StatePoisoned,
    InvalidDescriptor(String),
    Language(LanguageServiceError),
    Frontend(FrontendServiceRouterError),
}

impl Display for FrontendLanguageServiceError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::StatePoisoned => f.write_str("language service state lock poisoned"),
            Self::InvalidDescriptor(message) => {
                write!(f, "invalid frontend language provider: {message}")
            }
            Self::Language(error) => Display::fmt(error, f),
            Self::Frontend(error) => Display::fmt(error, f),
        }
    }
}

impl std::error::Error for FrontendLanguageServiceError {}

impl From<LanguageServiceError> for FrontendLanguageServiceError {
    fn from(value: LanguageServiceError) -> Self {
        Self::Language(value)
    }
}

impl From<FrontendServiceRouterError> for FrontendLanguageServiceError {
    fn from(value: FrontendServiceRouterError) -> Self {
        Self::Frontend(value)
    }
}

impl FrontendLanguageServices {
    pub(super) fn reconcile(
        &self,
        workspace: &WorkspaceId,
        service: &LanguageServiceKind,
        configuration: &LanguageServiceConfiguration,
        router: &FrontendServiceRouter,
        live_managed: &BTreeMap<LanguageProviderId, u64>,
    ) -> Result<Option<ActiveLanguageProvider>, FrontendLanguageServiceError> {
        let live = router.live_providers()?;
        let mut state = self
            .state
            .lock()
            .map_err(|_| FrontendLanguageServiceError::StatePoisoned)?;
        let candidates = frontend_candidates(&mut state, live)?;
        let matching = candidates
            .iter()
            .filter(|candidate| candidate.candidate.service == *service)
            .cloned()
            .collect::<Vec<_>>();
        let active = state.manager.reconcile(
            workspace,
            service,
            configuration,
            matching.iter().map(|candidate| candidate.candidate.clone()),
            live_managed,
        );
        let key = (workspace.clone(), service.clone());
        match &active {
            Some(active) => {
                if let LanguageProviderSource::Frontend { connection } = active.source {
                    let selected = matching.iter().find(|candidate| {
                        candidate.candidate.provider == active.provider
                            && candidate.candidate.capabilities == active.capabilities
                            && matches!(
                                candidate.candidate.source,
                                LanguageProviderSource::Frontend { connection: alias }
                                    if alias == connection
                            )
                    });
                    let selected = selected.ok_or_else(|| {
                        FrontendLanguageServiceError::InvalidDescriptor(
                            "selected frontend provider lost its catalog binding".to_owned(),
                        )
                    })?;
                    state.bindings.insert(
                        key,
                        FrontendLanguageBinding {
                            connection: selected.connection,
                            frontend_provider: selected.frontend_provider.clone(),
                            epoch: active.epoch,
                        },
                    );
                } else {
                    state.bindings.remove(&key);
                }
            }
            None => {
                state.bindings.remove(&key);
            }
        }
        Ok(active)
    }

    pub(super) fn request(
        &self,
        workspace: &WorkspaceId,
        service: &LanguageServiceKind,
        configuration: &LanguageServiceConfiguration,
        router: &FrontendServiceRouter,
        method: String,
        params: Value,
    ) -> Result<Value, FrontendLanguageServiceError> {
        let active = self
            .reconcile(workspace, service, configuration, router, &BTreeMap::new())?
            .ok_or(LanguageServiceError::Unavailable)?;
        if !matches!(active.source, LanguageProviderSource::Frontend { .. }) {
            return Err(LanguageServiceError::Unavailable.into());
        }
        let (lease, binding) = {
            let state = self
                .state
                .lock()
                .map_err(|_| FrontendLanguageServiceError::StatePoisoned)?;
            let lease = state.manager.lease(workspace, service)?;
            let binding = state
                .bindings
                .get(&(workspace.clone(), service.clone()))
                .cloned()
                .ok_or(LanguageServiceError::ProviderChanged)?;
            if binding.epoch != lease.epoch {
                return Err(LanguageServiceError::ProviderChanged.into());
            }
            (lease, binding)
        };
        let result = router.request_connection(
            binding.connection,
            FrontendServiceRequest {
                provider: binding.frontend_provider,
                method,
                params,
            },
        )?;
        self.reconcile(workspace, service, configuration, router, &BTreeMap::new())?;
        self.state
            .lock()
            .map_err(|_| FrontendLanguageServiceError::StatePoisoned)?
            .manager
            .validate_lease(&lease)?;
        Ok(result)
    }

    // The managed-LSP slice consumes this seam when it subscribes to frontend
    // diagnostics and provider-state notifications. The current slice owns and
    // tests the source/epoch validation contract.
    #[cfg_attr(not(test), expect(dead_code))]
    pub(super) fn validate_notification(
        &self,
        workspace: &WorkspaceId,
        service: &LanguageServiceKind,
        configuration: &LanguageServiceConfiguration,
        router: &FrontendServiceRouter,
        inbound: &FrontendServiceInboundNotification,
    ) -> Result<u64, FrontendLanguageServiceError> {
        let active = self
            .reconcile(workspace, service, configuration, router, &BTreeMap::new())?
            .ok_or(LanguageServiceError::Unavailable)?;
        let state = self
            .state
            .lock()
            .map_err(|_| FrontendLanguageServiceError::StatePoisoned)?;
        let binding = state
            .bindings
            .get(&(workspace.clone(), service.clone()))
            .ok_or(LanguageServiceError::ProviderChanged)?;
        if binding.epoch != active.epoch
            || binding.connection != inbound.connection
            || binding.frontend_provider != inbound.notification.provider
        {
            return Err(LanguageServiceError::ProviderChanged.into());
        }
        Ok(active.epoch)
    }
}

fn frontend_candidates(
    state: &mut FrontendLanguageState,
    live: Vec<(FrontendConnectionId, FrontendServiceProviderDescriptor)>,
) -> Result<Vec<FrontendLanguageCandidate>, FrontendLanguageServiceError> {
    live.into_iter()
        .filter_map(|(connection, descriptor)| {
            match parse_frontend_descriptor(state, connection, descriptor) {
                Ok(Some(candidate)) => Some(Ok(candidate)),
                Ok(None) => None,
                Err(error) => Some(Err(error)),
            }
        })
        .collect()
}

fn parse_frontend_descriptor(
    state: &mut FrontendLanguageState,
    connection: FrontendConnectionId,
    descriptor: FrontendServiceProviderDescriptor,
) -> Result<Option<FrontendLanguageCandidate>, FrontendLanguageServiceError> {
    let id = descriptor.id.as_str();
    if !id.starts_with(PROVIDER_PREFIX) {
        return Ok(None);
    }
    let remainder = &id[PROVIDER_PREFIX.len()..];
    let Some((service, provider)) = remainder.split_once('/') else {
        return Err(FrontendLanguageServiceError::InvalidDescriptor(format!(
            "provider id {id:?} must use language/<service>/<provider>"
        )));
    };
    if service.trim().is_empty() || provider.trim().is_empty() {
        return Err(FrontendLanguageServiceError::InvalidDescriptor(format!(
            "provider id {id:?} has an empty service or provider identity"
        )));
    }
    let service = LanguageServiceKind::parse(service.to_owned()).map_err(|error| {
        FrontendLanguageServiceError::InvalidDescriptor(format!(
            "provider id {id:?} has an invalid service identity: {error}"
        ))
    })?;
    let provider = LanguageProviderId::parse(provider.to_owned()).map_err(|error| {
        FrontendLanguageServiceError::InvalidDescriptor(format!(
            "provider id {id:?} has an invalid provider identity: {error}"
        ))
    })?;
    let connection_alias = if let Some(alias) = state.connection_aliases.get(&connection) {
        *alias
    } else {
        state.next_connection_alias = state.next_connection_alias.saturating_add(1);
        let alias = state.next_connection_alias;
        state.connection_aliases.insert(connection, alias);
        alias
    };
    let capabilities = LanguageProviderCapabilities {
        requests: descriptor.capabilities.contains(REQUESTS),
        notifications: descriptor.capabilities.contains(NOTIFICATIONS),
        shared_diagnostics: descriptor.capabilities.contains(SHARED_DIAGNOSTICS),
        background_documents: descriptor.capabilities.contains(BACKGROUND_DOCUMENTS),
        dirty_buffers: descriptor.capabilities.contains(DIRTY_BUFFERS),
    };
    Ok(Some(FrontendLanguageCandidate {
        connection,
        frontend_provider: descriptor.id,
        candidate: LanguageProviderCandidate {
            service,
            provider,
            capabilities,
            source: LanguageProviderSource::Frontend {
                connection: connection_alias,
            },
        },
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use phenix_protocol::{FrontendServiceNotification, ServerMessage};
    use std::collections::BTreeSet;
    use std::sync::mpsc;
    use std::thread;

    fn workspace() -> WorkspaceId {
        WorkspaceId::parse("workspace:test").unwrap()
    }

    fn kind() -> LanguageServiceKind {
        LanguageServiceKind::parse("rust").unwrap()
    }

    fn descriptor(
        connection_provider: &str,
        capabilities: &[&str],
    ) -> FrontendServiceProviderDescriptor {
        FrontendServiceProviderDescriptor {
            id: FrontendServiceProviderId::parse(connection_provider).unwrap(),
            capabilities: capabilities
                .iter()
                .map(|value| (*value).to_owned())
                .collect::<BTreeSet<_>>(),
        }
    }

    fn connection(
        router: &FrontendServiceRouter,
        descriptor: FrontendServiceProviderDescriptor,
    ) -> (
        FrontendConnectionId,
        super::super::frontend_services::FrontendConnectionLease,
        mpsc::Receiver<ServerMessage>,
    ) {
        let (output, receiver) = mpsc::sync_channel(8);
        let lease = router.open_connection(output).unwrap();
        let id = lease.id();
        router.replace_providers(id, vec![descriptor]).unwrap();
        (id, lease, receiver)
    }

    #[test]
    fn live_catalog_drives_provider_selection_and_disconnect_failover() {
        let router = FrontendServiceRouter::default();
        let services = FrontendLanguageServices::default();
        let (_first_id, first, _first_output) =
            connection(&router, descriptor("language/rust/b", &[REQUESTS]));
        let configuration = LanguageServiceConfiguration::default();
        let selected = services
            .reconcile(
                &workspace(),
                &kind(),
                &configuration,
                &router,
                &BTreeMap::new(),
            )
            .unwrap()
            .unwrap();
        assert_eq!(selected.provider.as_str(), "b");
        let first_epoch = selected.epoch;
        let (_second_id, _second, _second_output) =
            connection(&router, descriptor("language/rust/a", &[REQUESTS]));
        let retained = services
            .reconcile(
                &workspace(),
                &kind(),
                &configuration,
                &router,
                &BTreeMap::new(),
            )
            .unwrap()
            .unwrap();
        assert_eq!(retained.provider.as_str(), "b");
        drop(first);
        let selected = services
            .reconcile(
                &workspace(),
                &kind(),
                &configuration,
                &router,
                &BTreeMap::new(),
            )
            .unwrap()
            .unwrap();
        assert_eq!(selected.provider.as_str(), "a");
        assert!(selected.epoch > first_epoch);
    }

    #[test]
    fn capability_loss_invalidates_current_provider() {
        let router = FrontendServiceRouter::default();
        let services = FrontendLanguageServices::default();
        let (connection, _lease, _output) =
            connection(&router, descriptor("language/rust/frontend", &[REQUESTS]));
        let mut configuration = LanguageServiceConfiguration::default();
        configuration.set_requirement(phenix_core::LanguageServiceRequirement {
            service: kind(),
            required_capabilities: LanguageProviderCapabilities {
                requests: true,
                ..LanguageProviderCapabilities::default()
            },
            preferred_provider: None,
        });
        let selected = services
            .reconcile(
                &workspace(),
                &kind(),
                &configuration,
                &router,
                &BTreeMap::new(),
            )
            .unwrap()
            .unwrap();
        router
            .replace_providers(connection, vec![descriptor("language/rust/frontend", &[])])
            .unwrap();
        assert!(services
            .reconcile(
                &workspace(),
                &kind(),
                &configuration,
                &router,
                &BTreeMap::new(),
            )
            .unwrap()
            .is_none());
        let state = services.state.lock().unwrap();
        assert_eq!(
            state
                .manager
                .validate_lease(&phenix_core::LanguageProviderLease {
                    workspace: workspace(),
                    service: kind(),
                    provider: selected.provider,
                    epoch: selected.epoch,
                }),
            Err(LanguageServiceError::ProviderChanged)
        );
    }

    #[test]
    fn wrong_connection_notification_is_rejected() {
        let router = FrontendServiceRouter::default();
        let services = FrontendLanguageServices::default();
        let (_selected_connection, _selected, _output) =
            connection(&router, descriptor("language/rust/a", &[NOTIFICATIONS]));
        let (other_connection, _other, _other_output) =
            connection(&router, descriptor("language/rust/a", &[NOTIFICATIONS]));
        let configuration = LanguageServiceConfiguration::default();
        services
            .reconcile(
                &workspace(),
                &kind(),
                &configuration,
                &router,
                &BTreeMap::new(),
            )
            .unwrap();
        let wrong = FrontendServiceInboundNotification {
            connection: other_connection,
            notification: FrontendServiceNotification {
                provider: FrontendServiceProviderId::parse("language/rust/a").unwrap(),
                method: "diagnostics".to_owned(),
                params: Value::Null,
            },
        };
        assert_eq!(
            services
                .validate_notification(&workspace(), &kind(), &configuration, &router, &wrong)
                .unwrap_err(),
            FrontendLanguageServiceError::Language(LanguageServiceError::ProviderChanged)
        );
    }

    #[test]
    fn request_is_bound_to_selected_epoch() {
        let router = FrontendServiceRouter::default();
        let services = FrontendLanguageServices::default();
        let (connection, _lease, output) =
            connection(&router, descriptor("language/rust/frontend", &[REQUESTS]));
        let configuration = LanguageServiceConfiguration::default();
        let request_services = services.clone();
        let request_router = router.clone();
        let request_configuration = configuration.clone();
        let call = thread::spawn(move || {
            request_services.request(
                &workspace(),
                &kind(),
                &request_configuration,
                &request_router,
                "definition".to_owned(),
                Value::Null,
            )
        });
        let ServerMessage::FrontendServiceRequest { id, .. } = output.recv().unwrap() else {
            panic!("expected frontend request");
        };
        router.replace_providers(connection, Vec::new()).unwrap();
        router
            .complete_response(
                connection,
                phenix_protocol::FrontendServiceResponse {
                    id,
                    response: phenix_protocol::FrontendServiceResponsePayload::Ok {
                        result: serde_json::json!({"ok": true}),
                    },
                },
            )
            .unwrap();
        assert_eq!(
            call.join().unwrap().unwrap_err(),
            FrontendLanguageServiceError::Language(LanguageServiceError::ProviderChanged)
        );
    }
}
