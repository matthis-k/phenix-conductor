use phenix_core::{
    Authority, EventBus, EventEnvelope, EventError, EventFailurePolicy, EventSubscription,
    EventTypeId, PluginId, SubscriptionId, SubscriptionSpec,
};
use std::sync::{Arc, Mutex};

fn plugin(value: &str) -> PluginId {
    PluginId::parse(value).unwrap()
}

fn event_type() -> EventTypeId {
    EventTypeId::parse("fixture.completed").unwrap()
}

fn subscription(value: &str) -> SubscriptionId {
    SubscriptionId::parse(value).unwrap()
}

fn spec(id: &str, dependencies: &[&str], failure_policy: EventFailurePolicy) -> SubscriptionSpec {
    SubscriptionSpec {
        id: subscription(id),
        owner: plugin(id),
        event_type: event_type(),
        event_version: 1,
        dependencies: dependencies.iter().map(|id| subscription(id)).collect(),
        failure_policy,
        required_authority: Authority::default(),
        maximum_authority: Authority::default(),
        kernel_policy_revision: 1,
    }
}

fn envelope() -> EventEnvelope {
    EventEnvelope {
        event_type: event_type(),
        version: 1,
        emitter: plugin("fixture.emitter"),
        causality_id: 1,
        kernel_policy_revision: 1,
        payload: Vec::new(),
    }
}

#[test]
fn fail_delivery_is_a_delivery_error_and_does_not_rollback_prior_listener_work() {
    let bus = EventBus::default();
    let completed = Arc::new(Mutex::new(Vec::new()));
    let first_completed = Arc::clone(&completed);

    bus.replace_subscriptions([
        EventSubscription {
            spec: spec("first", &[], EventFailurePolicy::Warn),
            handler: Arc::new(move |_: &EventEnvelope, _: &Authority| {
                first_completed.lock().unwrap().push("first");
                Ok(())
            }),
        },
        EventSubscription {
            spec: spec("second", &["first"], EventFailurePolicy::FailDelivery),
            handler: Arc::new(|_: &EventEnvelope, _: &Authority| Err("delivery failed".into())),
        },
    ])
    .unwrap();

    assert_eq!(
        bus.dispatch(&envelope(), &Authority::default()),
        Err(EventError::HandlerFailed {
            subscription: subscription("second"),
            message: "delivery failed".into(),
        })
    );
    assert_eq!(&*completed.lock().unwrap(), &["first"]);
}

#[test]
fn fail_delivery_has_the_canonical_wire_name() {
    assert_eq!(
        serde_json::to_value(EventFailurePolicy::FailDelivery).unwrap(),
        serde_json::Value::String("fail_delivery".into())
    );
}
