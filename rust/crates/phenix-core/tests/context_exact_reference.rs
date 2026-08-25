use phenix_core::{ContextResourceId, ContextRevision, ExactReference};

#[test]
fn context_exact_reference_names_resource_and_immutable_revision() {
    let resource_id = ContextResourceId::parse("project:development").unwrap();
    let revision = ContextRevision::parse("sha256:exact").unwrap();

    let reference = ExactReference::Context {
        resource_id: resource_id.clone(),
        revision: revision.clone(),
    };

    assert_eq!(
        reference.to_string(),
        "context:project:development@sha256:exact"
    );

    let encoded = serde_json::to_value(&reference).unwrap();
    assert_eq!(encoded["kind"], "context");
    assert_eq!(encoded["id"]["resource_id"], resource_id.as_str());
    assert_eq!(encoded["id"]["revision"], revision.as_str());
}
