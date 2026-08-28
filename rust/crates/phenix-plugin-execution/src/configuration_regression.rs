use crate::{AgentDefinition, ExecutionConfigurationCommand, OrchestrationDefinition};
use serde_json::json;

fn agent() -> serde_json::Value {
    json!({
        "id": "agent.scout",
        "kind": "agent",
        "description": "Inspect repository evidence.",
        "input_schema": {"type": "string"},
        "output_schema": {"type": "string"},
        "capabilities": ["workspace.read"],
        "policy": {"requires_permission": false}
    })
}

fn orchestration() -> serde_json::Value {
    json!({
        "descriptor": {
            "id": "orchestration.review",
            "kind": "orchestration",
            "description": "Review independently.",
            "input_schema": {"type": "string"},
            "output_schema": {"type": "string"},
            "capabilities": [],
            "policy": {"requires_permission": false}
        },
        "policy": "sequential",
        "nodes": [{
            "callable": "agent.scout",
            "objective": "Inspect the current objective."
        }]
    })
}

#[test]
fn agent_local_invariants_are_parsed_from_wire_data() {
    let valid: AgentDefinition = serde_json::from_value(agent()).unwrap();
    assert_eq!(valid.id().as_str(), "agent.scout");
    assert_eq!(valid.description(), "Inspect repository evidence.");

    let mut invalid = agent();
    invalid["id"] = json!("  ");
    assert!(serde_json::from_value::<AgentDefinition>(invalid).is_err());

    let mut invalid = agent();
    invalid["kind"] = json!("orchestration");
    assert!(serde_json::from_value::<AgentDefinition>(invalid).is_err());

    let mut invalid = agent();
    invalid["description"] = json!("\n");
    assert!(serde_json::from_value::<AgentDefinition>(invalid).is_err());

    let mut invalid = agent();
    invalid["capabilities"] = json!(["has space"]);
    assert!(serde_json::from_value::<AgentDefinition>(invalid).is_err());
}

#[test]
fn orchestration_local_invariants_are_parsed_from_wire_data() {
    let valid: OrchestrationDefinition = serde_json::from_value(orchestration()).unwrap();
    assert_eq!(valid.id().as_str(), "orchestration.review");
    assert_eq!(valid.nodes().len(), 1);

    let mut invalid = orchestration();
    invalid["descriptor"]["kind"] = json!("agent");
    assert!(serde_json::from_value::<OrchestrationDefinition>(invalid).is_err());

    let mut invalid = orchestration();
    invalid["policy"] = json!("parallel");
    assert!(serde_json::from_value::<OrchestrationDefinition>(invalid).is_err());

    let mut invalid = orchestration();
    invalid["nodes"] = json!([]);
    assert!(serde_json::from_value::<OrchestrationDefinition>(invalid).is_err());

    let mut invalid = orchestration();
    invalid["nodes"][0]["callable"] = json!("");
    assert!(serde_json::from_value::<OrchestrationDefinition>(invalid).is_err());

    let mut invalid = orchestration();
    invalid["nodes"][0]["objective"] = json!("  ");
    assert!(serde_json::from_value::<OrchestrationDefinition>(invalid).is_err());
}

#[test]
fn command_ids_are_rejected_during_deserialization() {
    assert!(
        serde_json::from_value::<ExecutionConfigurationCommand>(json!({
            "operation": "get_agent",
            "id": ""
        }))
        .is_err()
    );
    assert!(
        serde_json::from_value::<ExecutionConfigurationCommand>(json!({
            "operation": "get_orchestration",
            "id": "\t"
        }))
        .is_err()
    );
}
