use crate::{Authority, EventTypeId, PluginId, SubscriptionId};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    fmt::{self, Display, Formatter},
    sync::{
        mpsc::{self, Receiver, Sender},
        Arc, Mutex,
    },
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum KernelEvent {
    PluginActivated(PluginId),
    PluginStopped(PluginId),
    TaskCancelled(u64),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EventEnvelope {
    pub event_type: EventTypeId,
    pub version: u32,
    pub emitter: PluginId,
    pub causality_id: u64,
    pub kernel_policy_revision: u64,
    pub payload: Vec<u8>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EventFailurePolicy {
    Ignore,
    Warn,
    FailOperation,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SubscriptionSpec {
    pub id: SubscriptionId,
    pub owner: PluginId,
    pub event_type: EventTypeId,
    pub event_version: u32,
    pub dependencies: Vec<SubscriptionId>,
    pub failure_policy: EventFailurePolicy,
    pub required_authority: Authority,
    pub maximum_authority: Authority,
    pub kernel_policy_revision: u64,
}

pub trait EventHandler: Send + Sync {
    fn handle(&self, event: &EventEnvelope, authority: &Authority) -> Result<(), String>;

    #[doc(hidden)]
    fn handle_with_bus(
        &self,
        _bus: &EventBus,
        event: &EventEnvelope,
        authority: &Authority,
    ) -> Result<(), String> {
        self.handle(event, authority)
    }
}

impl<F> EventHandler for F
where
    F: Fn(&EventEnvelope, &Authority) -> Result<(), String> + Send + Sync,
{
    fn handle(&self, event: &EventEnvelope, authority: &Authority) -> Result<(), String> {
        self(event, authority)
    }
}

#[derive(Clone)]
pub struct EventSubscription {
    pub spec: SubscriptionSpec,
    pub handler: Arc<dyn EventHandler>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct EventDispatchReport {
    pub delivered: Vec<SubscriptionId>,
    pub warnings: Vec<(SubscriptionId, String)>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EventError {
    DuplicateSubscription(SubscriptionId),
    CrossEventDependency {
        subscription: SubscriptionId,
        dependency: SubscriptionId,
    },
    UnknownDependency {
        subscription: SubscriptionId,
        dependency: SubscriptionId,
    },
    DependencyCycle(SubscriptionId),
    CausalReentry(SubscriptionId),
    AuthorityDenied(SubscriptionId),
    HandlerFailed {
        subscription: SubscriptionId,
        message: String,
    },
}

impl Display for EventError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateSubscription(id) => write!(f, "duplicate event subscription: {id}"),
            Self::CrossEventDependency {
                subscription,
                dependency,
            } => write!(
                f,
                "event subscription {subscription} has cross-event dependency {dependency}"
            ),
            Self::UnknownDependency {
                subscription,
                dependency,
            } => write!(
                f,
                "event subscription {subscription} depends on unknown subscription {dependency}"
            ),
            Self::DependencyCycle(id) => write!(f, "event subscription cycle includes {id}"),
            Self::CausalReentry(id) => write!(f, "event subscription causal re-entry: {id}"),
            Self::AuthorityDenied(id) => write!(f, "event subscription authority denied: {id}"),
            Self::HandlerFailed {
                subscription,
                message,
            } => write!(f, "event subscription {subscription} failed: {message}"),
        }
    }
}

impl Error for EventError {}

#[derive(Default)]
pub struct EventBus {
    kernel_subscribers: Mutex<Vec<Sender<KernelEvent>>>,
    subscriptions: Mutex<BTreeMap<SubscriptionId, EventSubscription>>,
    active_causality: Mutex<BTreeMap<u64, BTreeSet<SubscriptionId>>>,
}

impl fmt::Debug for EventBus {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("EventBus").finish_non_exhaustive()
    }
}

impl EventBus {
    pub fn validate_subscriptions(
        subscriptions: impl IntoIterator<Item = EventSubscription>,
    ) -> Result<(), EventError> {
        index_subscriptions(subscriptions)
            .and_then(|subscriptions| validate_dependencies(&subscriptions))
    }

    pub fn subscribe(&self) -> Receiver<KernelEvent> {
        let (sender, receiver) = mpsc::channel();
        self.kernel_subscribers
            .lock()
            .expect("event subscriber lock poisoned")
            .push(sender);
        receiver
    }

    pub fn publish(&self, event: KernelEvent) {
        self.kernel_subscribers
            .lock()
            .expect("event subscriber lock poisoned")
            .retain(|sender| sender.send(event.clone()).is_ok());
    }

    pub fn replace_subscriptions(
        &self,
        subscriptions: impl IntoIterator<Item = EventSubscription>,
    ) -> Result<(), EventError> {
        let indexed = index_subscriptions(subscriptions)?;
        validate_dependencies(&indexed)?;
        *self
            .subscriptions
            .lock()
            .expect("event subscription lock poisoned") = indexed;
        Ok(())
    }

    pub fn install_subscriptions(
        &self,
        subscriptions: impl IntoIterator<Item = EventSubscription>,
    ) -> Result<Vec<SubscriptionId>, EventError> {
        let mut current = self
            .subscriptions
            .lock()
            .expect("event subscription lock poisoned");
        let mut candidate = current.clone();
        let mut installed = Vec::new();
        for subscription in subscriptions {
            let id = subscription.spec.id.clone();
            if candidate.insert(id.clone(), subscription).is_some() {
                return Err(EventError::DuplicateSubscription(id));
            }
            installed.push(id);
        }
        validate_dependencies(&candidate)?;
        *current = candidate;
        Ok(installed)
    }

    pub fn remove_subscriptions(
        &self,
        subscriptions: impl IntoIterator<Item = SubscriptionId>,
    ) -> Result<(), EventError> {
        let mut current = self
            .subscriptions
            .lock()
            .expect("event subscription lock poisoned");
        let mut candidate = current.clone();
        for subscription in subscriptions {
            candidate.remove(&subscription);
        }
        validate_dependencies(&candidate)?;
        *current = candidate;
        Ok(())
    }

    pub fn dispatch(
        &self,
        event: &EventEnvelope,
        emitter_authority: &Authority,
    ) -> Result<EventDispatchReport, EventError> {
        let subscriptions = self
            .subscriptions
            .lock()
            .expect("event subscription lock poisoned")
            .clone();
        let order = dependency_order(&subscriptions, &event.event_type, event.version)?;
        let mut report = EventDispatchReport::default();

        for id in order {
            let subscription = &subscriptions[&id];
            if !emitter_authority.permits_all(&subscription.spec.required_authority) {
                return Err(EventError::AuthorityDenied(id));
            }
            {
                let mut active = self
                    .active_causality
                    .lock()
                    .expect("event causality lock poisoned");
                let active_for_cause = active.entry(event.causality_id).or_default();
                if !active_for_cause.insert(id.clone()) {
                    return Err(EventError::CausalReentry(id));
                }
            }

            let handler_authority =
                emitter_authority.attenuate(&subscription.spec.maximum_authority);
            let result = subscription
                .handler
                .handle_with_bus(self, event, &handler_authority);
            let mut active = self
                .active_causality
                .lock()
                .expect("event causality lock poisoned");
            if let Some(active_for_cause) = active.get_mut(&event.causality_id) {
                active_for_cause.remove(&id);
                if active_for_cause.is_empty() {
                    active.remove(&event.causality_id);
                }
            }

            match result {
                Ok(()) => report.delivered.push(id),
                Err(_) if subscription.spec.failure_policy == EventFailurePolicy::Ignore => {
                    report.delivered.push(id);
                }
                Err(message) if subscription.spec.failure_policy == EventFailurePolicy::Warn => {
                    report.delivered.push(id.clone());
                    report.warnings.push((id, message));
                }
                Err(message) => {
                    return Err(EventError::HandlerFailed {
                        subscription: id,
                        message,
                    });
                }
            }
        }

        Ok(report)
    }
}

fn index_subscriptions(
    subscriptions: impl IntoIterator<Item = EventSubscription>,
) -> Result<BTreeMap<SubscriptionId, EventSubscription>, EventError> {
    let mut indexed = BTreeMap::new();
    for subscription in subscriptions {
        let id = subscription.spec.id.clone();
        if indexed.insert(id.clone(), subscription).is_some() {
            return Err(EventError::DuplicateSubscription(id));
        }
    }
    Ok(indexed)
}

fn validate_dependencies(
    subscriptions: &BTreeMap<SubscriptionId, EventSubscription>,
) -> Result<(), EventError> {
    for subscription in subscriptions.values() {
        for dependency in &subscription.spec.dependencies {
            let Some(target) = subscriptions.get(dependency) else {
                return Err(EventError::UnknownDependency {
                    subscription: subscription.spec.id.clone(),
                    dependency: dependency.clone(),
                });
            };
            if target.spec.event_type != subscription.spec.event_type
                || target.spec.event_version != subscription.spec.event_version
            {
                return Err(EventError::CrossEventDependency {
                    subscription: subscription.spec.id.clone(),
                    dependency: dependency.clone(),
                });
            }
        }
    }

    for event in subscriptions
        .values()
        .map(|subscription| {
            (
                subscription.spec.event_type.clone(),
                subscription.spec.event_version,
            )
        })
        .collect::<BTreeSet<_>>()
    {
        dependency_order(subscriptions, &event.0, event.1)?;
    }
    Ok(())
}

fn dependency_order(
    subscriptions: &BTreeMap<SubscriptionId, EventSubscription>,
    event_type: &EventTypeId,
    event_version: u32,
) -> Result<Vec<SubscriptionId>, EventError> {
    #[derive(Clone, Copy, Eq, PartialEq)]
    enum Visit {
        Visiting,
        Done,
    }

    fn visit(
        id: &SubscriptionId,
        subscriptions: &BTreeMap<SubscriptionId, EventSubscription>,
        selected: &BTreeSet<SubscriptionId>,
        visits: &mut BTreeMap<SubscriptionId, Visit>,
        order: &mut Vec<SubscriptionId>,
    ) -> Result<(), EventError> {
        match visits.get(id) {
            Some(Visit::Done) => return Ok(()),
            Some(Visit::Visiting) => return Err(EventError::DependencyCycle(id.clone())),
            None => {}
        }
        visits.insert(id.clone(), Visit::Visiting);
        for dependency in &subscriptions[id].spec.dependencies {
            if selected.contains(dependency) {
                visit(dependency, subscriptions, selected, visits, order)?;
            }
        }
        visits.insert(id.clone(), Visit::Done);
        order.push(id.clone());
        Ok(())
    }

    let selected: BTreeSet<_> = subscriptions
        .values()
        .filter(|subscription| {
            &subscription.spec.event_type == event_type
                && subscription.spec.event_version == event_version
        })
        .map(|subscription| subscription.spec.id.clone())
        .collect();
    let mut visits = BTreeMap::new();
    let mut order = Vec::new();
    for id in &selected {
        visit(id, subscriptions, &selected, &mut visits, &mut order)?;
    }
    Ok(order)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::CapabilityId;
    use std::sync::Mutex;

    fn plugin(value: &str) -> PluginId {
        PluginId::parse(value).unwrap()
    }

    fn event_type(value: &str) -> EventTypeId {
        EventTypeId::parse(value).unwrap()
    }

    fn subscription(value: &str) -> SubscriptionId {
        SubscriptionId::parse(value).unwrap()
    }

    fn capability(value: &str) -> CapabilityId {
        CapabilityId::parse(value).unwrap()
    }

    fn envelope(causality_id: u64) -> EventEnvelope {
        EventEnvelope {
            event_type: event_type("demo.changed"),
            version: 1,
            emitter: plugin("emitter"),
            causality_id,
            kernel_policy_revision: 7,
            payload: b"payload".to_vec(),
        }
    }

    fn spec(id: &str, dependencies: &[&str]) -> SubscriptionSpec {
        SubscriptionSpec {
            id: subscription(id),
            owner: plugin(id),
            event_type: event_type("demo.changed"),
            event_version: 1,
            dependencies: dependencies.iter().map(|id| subscription(id)).collect(),
            failure_policy: EventFailurePolicy::FailOperation,
            required_authority: Authority::default(),
            maximum_authority: Authority::default(),
            kernel_policy_revision: 7,
        }
    }

    fn subscription_with(id: &str, dependencies: &[&str]) -> EventSubscription {
        EventSubscription {
            spec: spec(id, dependencies),
            handler: Arc::new(|_: &EventEnvelope, _: &Authority| Ok(())),
        }
    }

    #[test]
    fn kernel_subscribers_receive_generic_kernel_events() {
        let bus = EventBus::default();
        let receiver = bus.subscribe();
        let event = KernelEvent::TaskCancelled(7);
        bus.publish(event.clone());
        assert_eq!(receiver.recv().unwrap(), event);
    }

    #[test]
    fn all_subscribers_receive_event_in_dependency_order() {
        let bus = EventBus::default();
        let seen = Arc::new(Mutex::new(Vec::new()));
        let seen_a = Arc::clone(&seen);
        let seen_b = Arc::clone(&seen);
        bus.replace_subscriptions([
            EventSubscription {
                spec: spec("second", &["first"]),
                handler: Arc::new(move |_: &EventEnvelope, _: &Authority| {
                    seen_b.lock().unwrap().push("second");
                    Ok(())
                }),
            },
            EventSubscription {
                spec: spec("first", &[]),
                handler: Arc::new(move |_: &EventEnvelope, _: &Authority| {
                    seen_a.lock().unwrap().push("first");
                    Ok(())
                }),
            },
        ])
        .unwrap();

        let report = bus.dispatch(&envelope(1), &Authority::default()).unwrap();
        assert_eq!(&*seen.lock().unwrap(), &["first", "second"]);
        assert_eq!(
            report.delivered,
            vec![subscription("first"), subscription("second")]
        );
    }

    #[test]
    fn dependency_cycles_are_rejected_atomically() {
        let bus = EventBus::default();
        let noop: Arc<dyn EventHandler> = Arc::new(|_: &EventEnvelope, _: &Authority| Ok(()));
        let error = bus
            .replace_subscriptions([
                EventSubscription {
                    spec: spec("a", &["b"]),
                    handler: Arc::clone(&noop),
                },
                EventSubscription {
                    spec: spec("b", &["a"]),
                    handler: noop,
                },
            ])
            .unwrap_err();
        assert!(matches!(error, EventError::DependencyCycle(_)));
    }

    #[test]
    fn installing_subscriptions_preserves_existing_entries_and_is_atomic() {
        let bus = EventBus::default();
        bus.replace_subscriptions([subscription_with("first", &[])])
            .unwrap();
        let installed = bus
            .install_subscriptions([subscription_with("second", &["first"])])
            .unwrap();
        assert_eq!(installed, vec![subscription("second")]);

        let duplicate = bus
            .install_subscriptions([
                subscription_with("third", &[]),
                subscription_with("second", &[]),
            ])
            .unwrap_err();
        assert_eq!(
            duplicate,
            EventError::DuplicateSubscription(subscription("second"))
        );

        let report = bus.dispatch(&envelope(11), &Authority::default()).unwrap();
        assert_eq!(
            report.delivered,
            vec![subscription("first"), subscription("second")]
        );
    }

    #[test]
    fn removing_subscriptions_is_atomic_when_dependencies_would_break() {
        let bus = EventBus::default();
        bus.replace_subscriptions([
            subscription_with("first", &[]),
            subscription_with("second", &["first"]),
        ])
        .unwrap();

        let error = bus
            .remove_subscriptions([subscription("first")])
            .unwrap_err();
        assert_eq!(
            error,
            EventError::UnknownDependency {
                subscription: subscription("second"),
                dependency: subscription("first"),
            }
        );

        let report = bus.dispatch(&envelope(12), &Authority::default()).unwrap();
        assert_eq!(
            report.delivered,
            vec![subscription("first"), subscription("second")]
        );
    }

    #[test]
    fn event_handler_authority_is_attenuated() {
        let bus = EventBus::default();
        let read = capability("fs.read");
        let write = capability("fs.write");
        let mut handler_spec = spec("handler", &[]);
        handler_spec.required_authority = Authority::new([read.clone()]);
        handler_spec.maximum_authority = Authority::new([read.clone()]);
        bus.replace_subscriptions([EventSubscription {
            spec: handler_spec,
            handler: Arc::new(move |_: &EventEnvelope, authority: &Authority| {
                assert!(authority.permits(&read));
                assert!(!authority.permits(&write));
                Ok(())
            }),
        }])
        .unwrap();

        bus.dispatch(
            &envelope(2),
            &Authority::new([capability("fs.read"), capability("fs.write")]),
        )
        .unwrap();
    }

    #[test]
    fn same_subscription_causal_reentry_is_blocked() {
        let bus = Arc::new(EventBus::default());
        let nested_bus = Arc::clone(&bus);
        bus.replace_subscriptions([EventSubscription {
            spec: spec("recursive", &[]),
            handler: Arc::new(move |event: &EventEnvelope, authority: &Authority| {
                assert_eq!(
                    nested_bus.dispatch(event, authority),
                    Err(EventError::CausalReentry(subscription("recursive")))
                );
                Ok(())
            }),
        }])
        .unwrap();

        bus.dispatch(&envelope(9), &Authority::default()).unwrap();
    }

    #[test]
    fn warn_policy_records_failure_without_veto() {
        let bus = EventBus::default();
        let mut warning = spec("warning", &[]);
        warning.failure_policy = EventFailurePolicy::Warn;
        bus.replace_subscriptions([EventSubscription {
            spec: warning,
            handler: Arc::new(|_: &EventEnvelope, _: &Authority| Err("expected".into())),
        }])
        .unwrap();

        let report = bus.dispatch(&envelope(10), &Authority::default()).unwrap();
        assert_eq!(
            report.warnings,
            vec![(subscription("warning"), "expected".into())]
        );
    }
}
