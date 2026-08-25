use phenix_core::{
    ContextDescriptor, ContextInjection, ContextInjectionLifetime, ContextInjectionRequester,
    ContextResourceId, ContextResourceKind, ContextResourceRevision, ContextRevision, ContextScope,
    ContextTier, ExactReference, ExecutionId, WorkspaceId,
};
use std::path::PathBuf;

#[test]
fn context_revision_round_trip_preserves_exact_identity() {
    let id = ContextResourceId::parse("project:development").unwrap();
    let revision = ContextRevision::parse("sha256:development-v1").unwrap();
    let resource = ContextResourceRevision {
        descriptor: ContextDescriptor {
            id: id.clone(), kind: ContextResourceKind::ProjectDocument,
            title: "Development".to_owned(), description: "Project development instructions".to_owned(),
            scope: ContextScope::Path { path: PathBuf::from("DEVELOPMENT.md") },
            revision: revision.clone(), estimated_cost: 17,
        },
        tier: ContextTier::DiscoverableContent,
        source_ref: ExactReference::Context { resource_id: id, revision: revision.clone() },
        content_identity: revision, content: Some("development rules".to_owned()),
    };
    let encoded = serde_json::to_string(&resource).unwrap();
    let decoded: ContextResourceRevision = serde_json::from_str(&encoded).unwrap();
    assert_eq!(decoded, resource);
}

#[test]
fn context_injection_round_trip_preserves_requester_lifetime_and_revision() {
    let revision = ContextRevision::parse("sha256:development-v1").unwrap();
    let injection = ContextInjection {
        execution_id: ExecutionId::parse("execution-7").unwrap(),
        source_ref: ExactReference::Context {
            resource_id: ContextResourceId::parse("project:development").unwrap(),
            revision: revision.clone(),
        },
        source_revision: revision.clone(), requested_by: ContextInjectionRequester::Agent,
        reason: "load project development instructions".to_owned(),
        lifetime: ContextInjectionLifetime::Execution, content_identity: revision,
    };
    let encoded = serde_json::to_value(&injection).unwrap();
    let decoded: ContextInjection = serde_json::from_value(encoded).unwrap();
    assert_eq!(decoded, injection);
}

#[test]
fn workspace_scoped_context_round_trip_keeps_scope_identity() {
    let descriptor = ContextDescriptor {
        id: ContextResourceId::parse("project:contributing").unwrap(),
        kind: ContextResourceKind::ProjectDocument, title: "Contributing".to_owned(),
        description: "Contribution instructions".to_owned(),
        scope: ContextScope::Workspace { workspace_id: WorkspaceId::parse("workspace-3").unwrap() },
        revision: ContextRevision::parse("sha256:contributing-v1").unwrap(), estimated_cost: 12,
    };
    let encoded = serde_json::to_string(&descriptor).unwrap();
    let decoded: ContextDescriptor = serde_json::from_str(&encoded).unwrap();
    assert_eq!(decoded, descriptor);
}

#[test]
fn context_exact_reference_names_resource_and_immutable_revision() {
    let resource_id = ContextResourceId::parse("project:development").unwrap();
    let revision = ContextRevision::parse("sha256:exact").unwrap();
    let reference = ExactReference::Context { resource_id: resource_id.clone(), revision: revision.clone() };
    assert_eq!(reference.to_string(), "context:project:development@sha256:exact");
    let encoded = serde_json::to_value(&reference).unwrap();
    assert_eq!(encoded["kind"], "context");
    assert_eq!(encoded["id"]["resource_id"], resource_id.as_str());
    assert_eq!(encoded["id"]["revision"], revision.as_str());
}
