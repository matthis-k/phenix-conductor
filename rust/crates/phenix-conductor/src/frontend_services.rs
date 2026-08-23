use phenix_core::ExecutionId;
use phenix_protocol::{
    FrontendServiceError, FrontendServiceNotification, FrontendServiceProviderDescriptor,
    FrontendServiceProviderId, FrontendServiceRequest, FrontendServiceResponse,
    FrontendServiceResponsePayload, ServerMessage,
};
use serde_json::Value;
use std::collections::BTreeMap;
use std::fmt::{self, Display, Formatter};
use std::sync::{
    atomic::{AtomicU64, Ordering},
    mpsc::{self, Receiver, Sender, SyncSender},
    Arc, Mutex,
};

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(super) struct FrontendConnectionId(u64);

#[derive(Clone, Debug, PartialEq)]
pub(super) struct FrontendServiceInboundNotification {
    pub connection: FrontendConnectionId,
    pub notification: FrontendServiceNotification,
}

#[derive(Clone, Debug, PartialEq)]
pub(super) enum FrontendServiceRouterError {
    StatePoisoned,
    UnknownConnection,
    DuplicateProvider(FrontendServiceProviderId),
    ExecutionAlreadyOwned(ExecutionId),
    NoFrontendForExecution(ExecutionId),
    ProviderUnavailable(FrontendServiceProviderId),
    OutputClosed,
    Disconnected,
    UnknownRequest(u64),
    WrongConnection(u64),
    Remote(FrontendServiceError),
}

impl Display for FrontendServiceRouterError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::StatePoisoned => f.write_str("frontend service router lock poisoned"),
            Self::UnknownConnection => f.write_str("frontend connection is no longer active"),
            Self::DuplicateProvider(id) => {
                write!(
                    f,
                    "frontend service provider advertised more than once: {id}"
                )
            }
            Self::ExecutionAlreadyOwned(id) => {
                write!(
                    f,
                    "root execution already belongs to another frontend: {id}"
                )
            }
            Self::NoFrontendForExecution(id) => {
                write!(f, "root execution has no live frontend service route: {id}")
            }
            Self::ProviderUnavailable(id) => {
                write!(f, "frontend service provider is not available: {id}")
            }
            Self::OutputClosed => f.write_str("frontend service output channel closed"),
            Self::Disconnected => f.write_str("frontend disconnected during service call"),
            Self::UnknownRequest(id) => write!(f, "unknown frontend service request: {id}"),
            Self::WrongConnection(id) => write!(
                f,
                "frontend service response came from the wrong connection: {id}"
            ),
            Self::Remote(error) => write!(f, "frontend service {}: {}", error.code, error.message),
        }
    }
}

impl std::error::Error for FrontendServiceRouterError {}

#[derive(Clone, Default)]
pub(super) struct FrontendServiceRouter {
    inner: Arc<FrontendServiceRouterInner>,
}

#[derive(Default)]
struct FrontendServiceRouterInner {
    next_connection: AtomicU64,
    next_request: AtomicU64,
    state: Mutex<FrontendServiceRouterState>,
}

#[derive(Default)]
struct FrontendServiceRouterState {
    connections: BTreeMap<FrontendConnectionId, FrontendConnection>,
    root_owners: BTreeMap<ExecutionId, FrontendConnectionId>,
    pending: BTreeMap<u64, PendingFrontendRequest>,
    notification_subscribers: Vec<Sender<FrontendServiceInboundNotification>>,
}

struct FrontendConnection {
    output: SyncSender<ServerMessage>,
    providers: BTreeMap<FrontendServiceProviderId, FrontendServiceProviderDescriptor>,
}

struct PendingFrontendRequest {
    connection: FrontendConnectionId,
    completion: Sender<Result<Value, FrontendServiceRouterError>>,
}

pub(super) struct FrontendConnectionLease {
    router: FrontendServiceRouter,
    id: FrontendConnectionId,
}

impl FrontendConnectionLease {
    #[must_use]
    pub(super) fn id(&self) -> FrontendConnectionId {
        self.id
    }
}

impl Drop for FrontendConnectionLease {
    fn drop(&mut self) {
        self.router.disconnect(self.id);
    }
}

impl FrontendServiceRouter {
    pub(super) fn open_connection(
        &self,
        output: SyncSender<ServerMessage>,
    ) -> Result<FrontendConnectionLease, FrontendServiceRouterError> {
        let id =
            FrontendConnectionId(self.inner.next_connection.fetch_add(1, Ordering::Relaxed) + 1);
        let mut state = self
            .inner
            .state
            .lock()
            .map_err(|_| FrontendServiceRouterError::StatePoisoned)?;
        state.connections.insert(
            id,
            FrontendConnection {
                output,
                providers: BTreeMap::new(),
            },
        );
        Ok(FrontendConnectionLease {
            router: self.clone(),
            id,
        })
    }

    pub(super) fn replace_providers(
        &self,
        connection: FrontendConnectionId,
        providers: Vec<FrontendServiceProviderDescriptor>,
    ) -> Result<(), FrontendServiceRouterError> {
        let mut indexed = BTreeMap::new();
        for provider in providers {
            let id = provider.id.clone();
            if indexed.insert(id.clone(), provider).is_some() {
                return Err(FrontendServiceRouterError::DuplicateProvider(id));
            }
        }
        let mut state = self
            .inner
            .state
            .lock()
            .map_err(|_| FrontendServiceRouterError::StatePoisoned)?;
        let registered = state
            .connections
            .get_mut(&connection)
            .ok_or(FrontendServiceRouterError::UnknownConnection)?;
        registered.providers = indexed;
        Ok(())
    }

    pub(super) fn live_providers(
        &self,
    ) -> Result<
        Vec<(FrontendConnectionId, FrontendServiceProviderDescriptor)>,
        FrontendServiceRouterError,
    > {
        let state = self
            .inner
            .state
            .lock()
            .map_err(|_| FrontendServiceRouterError::StatePoisoned)?;
        Ok(state
            .connections
            .iter()
            .flat_map(|(connection, frontend)| {
                frontend
                    .providers
                    .values()
                    .cloned()
                    .map(|provider| (*connection, provider))
            })
            .collect())
    }

    pub(super) fn subscribe_notifications(
        &self,
    ) -> Result<Receiver<FrontendServiceInboundNotification>, FrontendServiceRouterError> {
        let (sender, receiver) = mpsc::channel();
        self.inner
            .state
            .lock()
            .map_err(|_| FrontendServiceRouterError::StatePoisoned)?
            .notification_subscribers
            .push(sender);
        Ok(receiver)
    }

    pub(super) fn accept_notification(
        &self,
        connection: FrontendConnectionId,
        notification: FrontendServiceNotification,
    ) -> Result<(), FrontendServiceRouterError> {
        let mut state = self
            .inner
            .state
            .lock()
            .map_err(|_| FrontendServiceRouterError::StatePoisoned)?;
        let frontend = state
            .connections
            .get(&connection)
            .ok_or(FrontendServiceRouterError::UnknownConnection)?;
        if !frontend.providers.contains_key(&notification.provider) {
            return Err(FrontendServiceRouterError::ProviderUnavailable(
                notification.provider.clone(),
            ));
        }
        let event = FrontendServiceInboundNotification {
            connection,
            notification,
        };
        state
            .notification_subscribers
            .retain(|subscriber| subscriber.send(event.clone()).is_ok());
        Ok(())
    }

    pub(super) fn bind_execution(
        &self,
        connection: FrontendConnectionId,
        root: ExecutionId,
    ) -> Result<(), FrontendServiceRouterError> {
        let mut state = self
            .inner
            .state
            .lock()
            .map_err(|_| FrontendServiceRouterError::StatePoisoned)?;
        if !state.connections.contains_key(&connection) {
            return Err(FrontendServiceRouterError::UnknownConnection);
        }
        match state.root_owners.get(&root) {
            Some(owner) if owner != &connection => {
                return Err(FrontendServiceRouterError::ExecutionAlreadyOwned(root));
            }
            Some(_) => return Ok(()),
            None => {}
        }
        state.root_owners.insert(root, connection);
        Ok(())
    }

    pub(super) fn release_execution(
        &self,
        root: &ExecutionId,
    ) -> Result<(), FrontendServiceRouterError> {
        self.inner
            .state
            .lock()
            .map_err(|_| FrontendServiceRouterError::StatePoisoned)?
            .root_owners
            .remove(root);
        Ok(())
    }

    pub(super) fn request(
        &self,
        root: &ExecutionId,
        request: FrontendServiceRequest,
    ) -> Result<Value, FrontendServiceRouterError> {
        let connection = {
            let state = self
                .inner
                .state
                .lock()
                .map_err(|_| FrontendServiceRouterError::StatePoisoned)?;
            *state
                .root_owners
                .get(root)
                .ok_or_else(|| FrontendServiceRouterError::NoFrontendForExecution(root.clone()))?
        };
        self.request_connection(connection, request)
    }

    pub(super) fn request_connection(
        &self,
        connection: FrontendConnectionId,
        request: FrontendServiceRequest,
    ) -> Result<Value, FrontendServiceRouterError> {
        let request_id = self.inner.next_request.fetch_add(1, Ordering::Relaxed) + 1;
        let (completion, receiver) = mpsc::channel();
        let output = {
            let mut state = self
                .inner
                .state
                .lock()
                .map_err(|_| FrontendServiceRouterError::StatePoisoned)?;
            let frontend = state
                .connections
                .get(&connection)
                .ok_or(FrontendServiceRouterError::Disconnected)?;
            if !frontend.providers.contains_key(&request.provider) {
                return Err(FrontendServiceRouterError::ProviderUnavailable(
                    request.provider.clone(),
                ));
            }
            let output = frontend.output.clone();
            state.pending.insert(
                request_id,
                PendingFrontendRequest {
                    connection,
                    completion,
                },
            );
            output
        };

        if output
            .send(ServerMessage::FrontendServiceRequest {
                id: request_id,
                request,
            })
            .is_err()
        {
            self.fail_pending(request_id, FrontendServiceRouterError::OutputClosed);
            return Err(FrontendServiceRouterError::OutputClosed);
        }

        receiver
            .recv()
            .unwrap_or(Err(FrontendServiceRouterError::Disconnected))
    }

    pub(super) fn notify(
        &self,
        root: &ExecutionId,
        notification: FrontendServiceNotification,
    ) -> Result<(), FrontendServiceRouterError> {
        let connection = {
            let state = self
                .inner
                .state
                .lock()
                .map_err(|_| FrontendServiceRouterError::StatePoisoned)?;
            *state
                .root_owners
                .get(root)
                .ok_or_else(|| FrontendServiceRouterError::NoFrontendForExecution(root.clone()))?
        };
        self.notify_connection(connection, notification)
    }

    pub(super) fn notify_connection(
        &self,
        connection: FrontendConnectionId,
        notification: FrontendServiceNotification,
    ) -> Result<(), FrontendServiceRouterError> {
        let output = {
            let state = self
                .inner
                .state
                .lock()
                .map_err(|_| FrontendServiceRouterError::StatePoisoned)?;
            let frontend = state
                .connections
                .get(&connection)
                .ok_or(FrontendServiceRouterError::Disconnected)?;
            if !frontend.providers.contains_key(&notification.provider) {
                return Err(FrontendServiceRouterError::ProviderUnavailable(
                    notification.provider.clone(),
                ));
            }
            frontend.output.clone()
        };
        output
            .send(ServerMessage::FrontendServiceNotification { notification })
            .map_err(|_| FrontendServiceRouterError::OutputClosed)
    }

    pub(super) fn complete_response(
        &self,
        connection: FrontendConnectionId,
        response: FrontendServiceResponse,
    ) -> Result<(), FrontendServiceRouterError> {
        let pending = {
            let mut state = self
                .inner
                .state
                .lock()
                .map_err(|_| FrontendServiceRouterError::StatePoisoned)?;
            let pending = state
                .pending
                .get(&response.id)
                .ok_or(FrontendServiceRouterError::UnknownRequest(response.id))?;
            if pending.connection != connection {
                return Err(FrontendServiceRouterError::WrongConnection(response.id));
            }
            state
                .pending
                .remove(&response.id)
                .expect("validated pending frontend request remains present")
        };
        let result = match response.response {
            FrontendServiceResponsePayload::Ok { result } => Ok(result),
            FrontendServiceResponsePayload::Error { error } => {
                Err(FrontendServiceRouterError::Remote(error))
            }
        };
        let _ = pending.completion.send(result);
        Ok(())
    }

    fn fail_pending(&self, request_id: u64, error: FrontendServiceRouterError) {
        let pending = self
            .inner
            .state
            .lock()
            .ok()
            .and_then(|mut state| state.pending.remove(&request_id));
        if let Some(pending) = pending {
            let _ = pending.completion.send(Err(error));
        }
    }

    fn disconnect(&self, connection: FrontendConnectionId) {
        let pending = {
            let Ok(mut state) = self.inner.state.lock() else {
                return;
            };
            state.connections.remove(&connection);
            state.root_owners.retain(|_, owner| owner != &connection);
            let request_ids = state
                .pending
                .iter()
                .filter_map(|(id, pending)| (pending.connection == connection).then_some(*id))
                .collect::<Vec<_>>();
            request_ids
                .into_iter()
                .filter_map(|id| state.pending.remove(&id))
                .collect::<Vec<_>>()
        };
        for pending in pending {
            let _ = pending
                .completion
                .send(Err(FrontendServiceRouterError::Disconnected));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;
    use std::sync::mpsc::Receiver;
    use std::thread;
    use std::time::Duration;

    fn provider(id: &str) -> FrontendServiceProviderDescriptor {
        FrontendServiceProviderDescriptor {
            id: FrontendServiceProviderId::parse(id).unwrap(),
            capabilities: BTreeSet::from(["search".to_owned()]),
        }
    }

    fn request(provider: &str, method: &str) -> FrontendServiceRequest {
        FrontendServiceRequest {
            provider: FrontendServiceProviderId::parse(provider).unwrap(),
            method: method.to_owned(),
            params: serde_json::json!({"method": method}),
        }
    }

    fn notification(provider: &str, method: &str) -> FrontendServiceNotification {
        FrontendServiceNotification {
            provider: FrontendServiceProviderId::parse(provider).unwrap(),
            method: method.to_owned(),
            params: serde_json::json!({"method": method}),
        }
    }

    fn connection(
        router: &FrontendServiceRouter,
        providers: Vec<FrontendServiceProviderDescriptor>,
    ) -> (FrontendConnectionLease, Receiver<ServerMessage>) {
        let (sender, receiver) = mpsc::sync_channel(8);
        let lease = router.open_connection(sender).unwrap();
        router.replace_providers(lease.id(), providers).unwrap();
        (lease, receiver)
    }

    fn response(id: u64, value: Value) -> FrontendServiceResponse {
        FrontendServiceResponse {
            id,
            response: FrontendServiceResponsePayload::Ok { result: value },
        }
    }

    #[test]
    fn provider_catalog_preserves_connection_identity_and_capabilities() {
        let router = FrontendServiceRouter::default();
        let (first, _first_output) = connection(&router, vec![provider("web")]);
        let (second, _second_output) = connection(&router, vec![provider("editor")]);
        let providers = router.live_providers().unwrap();
        assert_eq!(providers.len(), 2);
        assert!(providers.iter().any(|(connection, descriptor)| {
            *connection == first.id()
                && descriptor.id.as_str() == "web"
                && descriptor.capabilities.contains("search")
        }));
        assert!(providers.iter().any(|(connection, descriptor)| {
            *connection == second.id() && descriptor.id.as_str() == "editor"
        }));
    }

    #[test]
    fn inbound_notifications_validate_provider_and_preserve_source_connection() {
        let router = FrontendServiceRouter::default();
        let (lease, _output) = connection(&router, vec![provider("web")]);
        let events = router.subscribe_notifications().unwrap();
        router
            .accept_notification(lease.id(), notification("web", "changed"))
            .unwrap();
        let event = events.recv().unwrap();
        assert_eq!(event.connection, lease.id());
        assert_eq!(event.notification.method, "changed");
        assert_eq!(
            router
                .accept_notification(lease.id(), notification("missing", "changed"))
                .unwrap_err(),
            FrontendServiceRouterError::ProviderUnavailable(
                FrontendServiceProviderId::parse("missing").unwrap()
            )
        );
    }

    #[test]
    fn direct_connection_requests_support_workspace_owned_integrations() {
        let router = FrontendServiceRouter::default();
        let (lease, output) = connection(&router, vec![provider("web")]);
        let request_router = router.clone();
        let connection = lease.id();
        let call = thread::spawn(move || {
            request_router.request_connection(connection, request("web", "search"))
        });
        let ServerMessage::FrontendServiceRequest { id, .. } = output.recv().unwrap() else {
            panic!("expected frontend service request");
        };
        router
            .complete_response(lease.id(), response(id, serde_json::json!("ok")))
            .unwrap();
        assert_eq!(call.join().unwrap().unwrap(), serde_json::json!("ok"));
    }

    #[test]
    fn concurrent_requests_are_correlated_even_when_responses_arrive_out_of_order() {
        let router = FrontendServiceRouter::default();
        let (lease, output) = connection(&router, vec![provider("web")]);
        let first_root = ExecutionId::parse("execution-1").unwrap();
        let second_root = ExecutionId::parse("execution-2").unwrap();
        router
            .bind_execution(lease.id(), first_root.clone())
            .unwrap();
        router
            .bind_execution(lease.id(), second_root.clone())
            .unwrap();

        let first_router = router.clone();
        let first =
            thread::spawn(move || first_router.request(&first_root, request("web", "first")));
        let second_router = router.clone();
        let second =
            thread::spawn(move || second_router.request(&second_root, request("web", "second")));

        let mut calls = BTreeMap::new();
        for _ in 0..2 {
            let ServerMessage::FrontendServiceRequest { id, request } = output.recv().unwrap()
            else {
                panic!("expected frontend service request");
            };
            calls.insert(request.method, id);
        }
        router
            .complete_response(
                lease.id(),
                response(calls["second"], serde_json::json!("second-result")),
            )
            .unwrap();
        router
            .complete_response(
                lease.id(),
                response(calls["first"], serde_json::json!("first-result")),
            )
            .unwrap();

        assert_eq!(
            first.join().unwrap().unwrap(),
            serde_json::json!("first-result")
        );
        assert_eq!(
            second.join().unwrap().unwrap(),
            serde_json::json!("second-result")
        );
    }

    #[test]
    fn only_the_owning_frontend_receives_and_completes_a_request() {
        let router = FrontendServiceRouter::default();
        let (owner, owner_output) = connection(&router, vec![provider("web")]);
        let (other, other_output) = connection(&router, vec![provider("web")]);
        let root = ExecutionId::parse("execution-1").unwrap();
        router.bind_execution(owner.id(), root.clone()).unwrap();

        let request_router = router.clone();
        let call = thread::spawn(move || request_router.request(&root, request("web", "search")));
        let ServerMessage::FrontendServiceRequest { id, .. } = owner_output.recv().unwrap() else {
            panic!("expected frontend service request");
        };
        assert!(other_output
            .recv_timeout(Duration::from_millis(50))
            .is_err());
        assert_eq!(
            router
                .complete_response(other.id(), response(id, serde_json::json!("wrong")))
                .unwrap_err(),
            FrontendServiceRouterError::WrongConnection(id)
        );
        router
            .complete_response(owner.id(), response(id, serde_json::json!("right")))
            .unwrap();
        assert_eq!(call.join().unwrap().unwrap(), serde_json::json!("right"));
    }

    #[test]
    fn disconnect_removes_registrations_routes_and_pending_calls() {
        let router = FrontendServiceRouter::default();
        let (lease, output) = connection(&router, vec![provider("web")]);
        let root = ExecutionId::parse("execution-1").unwrap();
        router.bind_execution(lease.id(), root.clone()).unwrap();
        let request_router = router.clone();
        let request_root = root.clone();
        let call =
            thread::spawn(move || request_router.request(&request_root, request("web", "search")));
        let _ = output.recv().unwrap();
        drop(lease);
        assert_eq!(
            call.join().unwrap().unwrap_err(),
            FrontendServiceRouterError::Disconnected
        );
        assert_eq!(
            router.request(&root, request("web", "search")).unwrap_err(),
            FrontendServiceRouterError::NoFrontendForExecution(root)
        );
    }

    #[test]
    fn provider_updates_replace_the_connection_catalog() {
        let router = FrontendServiceRouter::default();
        let (lease, _output) = connection(&router, vec![provider("web")]);
        let root = ExecutionId::parse("execution-1").unwrap();
        router.bind_execution(lease.id(), root.clone()).unwrap();
        router
            .replace_providers(lease.id(), vec![provider("editor")])
            .unwrap();
        assert_eq!(
            router.request(&root, request("web", "search")).unwrap_err(),
            FrontendServiceRouterError::ProviderUnavailable(
                FrontendServiceProviderId::parse("web").unwrap()
            )
        );
    }

    #[test]
    fn new_router_has_no_frontend_state_after_restart() {
        let first = FrontendServiceRouter::default();
        let (lease, _output) = connection(&first, vec![provider("web")]);
        let root = ExecutionId::parse("execution-1").unwrap();
        first.bind_execution(lease.id(), root.clone()).unwrap();

        let restarted = FrontendServiceRouter::default();
        assert_eq!(
            restarted
                .request(&root, request("web", "search"))
                .unwrap_err(),
            FrontendServiceRouterError::NoFrontendForExecution(root)
        );
    }

    #[test]
    fn notification_uses_the_same_owner_and_provider_checks() {
        let router = FrontendServiceRouter::default();
        let (lease, output) = connection(&router, vec![provider("web")]);
        let root = ExecutionId::parse("execution-1").unwrap();
        router.bind_execution(lease.id(), root.clone()).unwrap();
        router
            .notify(&root, notification("web", "invalidate"))
            .unwrap();
        assert!(matches!(
            output.recv().unwrap(),
            ServerMessage::FrontendServiceNotification { .. }
        ));
    }
}
