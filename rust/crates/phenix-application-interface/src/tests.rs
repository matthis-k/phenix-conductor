use super::*;
use crate::{descriptor::id, types::*};
use futures::executor::block_on;
use phenix_core::{HasPhenixSchema, PhenixValue, SessionId, ValueCodec};
use std::{cell::Cell, rc::Rc};

#[allow(dead_code)]
mod generated {
    include!("../fixtures/application.rs");
}

const JSON: &str = include_str!("../../../../share/phenix/interfaces/phenix.application@1.json");
const RUST: &str = include_str!("../fixtures/application.rs");

#[test]
fn descriptor_and_compiled_rust_regenerate_from_the_same_snapshot() {
    let descriptor = application_descriptor();
    assert_eq!(descriptor.canonical_json().unwrap(), JSON);
    let decoded: ApplicationDescriptor = serde_json::from_str(JSON).unwrap();
    assert_eq!(decoded, descriptor);
    assert_eq!(generate::rust(&decoded).unwrap(), RUST);
    assert_eq!(generated::INTERFACE_ID, INTERFACE_ID);
    assert_eq!(generated::type_schemas(), descriptor.types);
}

#[test]
fn broken_descriptor_references_and_unsupported_shapes_fail_generation() {
    let mut descriptor = application_descriptor();
    descriptor
        .types
        .remove(&id("phenix.application.type.session-input@1"));
    assert!(matches!(
        generate::rust(&descriptor),
        Err(generate::GenerationError::MissingReference(_))
    ));
    let mut descriptor = application_descriptor();
    descriptor
        .types
        .insert(id("fixture.never@1"), phenix_core::PhenixSchema::Never);
    assert!(matches!(
        generate::rust(&descriptor),
        Err(generate::GenerationError::UnsupportedSchema(_))
    ));
}

fn all_capabilities() -> Capabilities {
    let descriptor = application_descriptor();
    Capabilities::negotiate(&descriptor, descriptor.capabilities.keys().cloned()).unwrap()
}

struct Sessions {
    calls: Rc<Cell<usize>>,
    prefix: &'static str,
}
impl ApplicationTransport for Sessions {
    async fn invoke(
        &self,
        operation: &phenix_core::ContractId,
        input: PhenixValue,
    ) -> Result<PhenixValue, ApplicationError> {
        self.calls.set(self.calls.get() + 1);
        assert_eq!(operation.as_str(), CreateSession::ID);
        let request = SessionCreateInput::from_value(&input).map_err(|error| {
            ApplicationError::InvalidInput {
                message: error.to_string(),
            }
        })?;
        Ok(SessionInfo {
            session_id: SessionId::parse(format!("{}-1", self.prefix)).unwrap(),
            title: request.title,
            working_directory: request.working_directory,
        }
        .to_value())
    }
}

#[test]
fn generated_typed_wrapper_crosses_a_replaceable_application_boundary() {
    use generated::{
        PhenixApplicationSessionCreate1Operation as Create,
        PhenixApplicationTypeSessionCreateInput1Type as Input,
    };
    for prefix in ["memory", "persistent"] {
        let calls = Rc::new(Cell::new(0));
        let client = ApplicationClient::new(
            Sessions {
                calls: calls.clone(),
                prefix,
            },
            all_capabilities(),
        );
        let response = block_on(Create::invoke(
            &client,
            Input {
                title: Some("Editor".into()),
                working_directory: "/workspace".into(),
            },
        ))
        .unwrap();
        assert_eq!(response.session_id, format!("{prefix}-1"));
        assert_eq!(response.title.as_deref(), Some("Editor"));
        assert_eq!(calls.get(), 1);
    }
}

#[test]
fn unavailable_capability_rejects_before_transport_invocation() {
    let calls = Rc::new(Cell::new(0));
    let client = ApplicationClient::new(
        Sessions {
            calls: calls.clone(),
            prefix: "unused",
        },
        Capabilities::default(),
    );
    let error = block_on(client.invoke::<CreateSession>(SessionCreateInput {
        title: None,
        working_directory: "/workspace".into(),
    }))
    .unwrap_err();
    assert_eq!(
        error,
        ApplicationError::UnsupportedCapability {
            capability: id(CreateSession::CAPABILITY)
        }
    );
    assert_eq!(calls.get(), 0);
}

#[test]
fn negotiation_ignores_unknown_versions_and_rejects_missing_dependencies() {
    let descriptor = application_descriptor();
    let capabilities = Capabilities::negotiate(&descriptor, [id("future.feature@99")]).unwrap();
    assert_eq!(capabilities.iter().count(), 0);
    assert!(matches!(
        Capabilities::negotiate(&descriptor, [id(Prompt::CAPABILITY)]),
        Err(ApplicationError::UnsupportedCapability { .. })
    ));
}

#[test]
fn malformed_runtime_responses_remain_typed_errors() {
    struct Broken;
    impl ApplicationTransport for Broken {
        async fn invoke(
            &self,
            _: &phenix_core::ContractId,
            _: PhenixValue,
        ) -> Result<PhenixValue, ApplicationError> {
            Ok(PhenixValue::String("not a session".into()))
        }
    }
    let client = ApplicationClient::new(Broken, all_capabilities());
    assert!(matches!(
        block_on(client.invoke::<CreateSession>(SessionCreateInput {
            title: None,
            working_directory: "/workspace".into(),
        })),
        Err(ApplicationError::InvalidResponse { .. })
    ));
}

#[test]
fn generated_callbacks_events_and_errors_preserve_structural_values() {
    use generated::{
        PhenixApplicationTypeExecutionUpdate1Type as Update,
        PhenixApplicationTypePermissionResponse1Type as Permission,
    };
    let event = ExecutionUpdate {
        session_id: SessionId::parse("editor").unwrap(),
        execution_id: "turn-1".into(),
        sequence: 7,
        update: ExecutionChange::ToolFailed {
            call_id: "call-1".into(),
            error: ApplicationError::PermissionDenied {
                message: "read only".into(),
            },
        },
    };
    let generated = Update::from_value(&event.to_value()).unwrap();
    assert_eq!(
        ExecutionUpdate::from_value(&generated.to_value()).unwrap(),
        event
    );
    assert_eq!(
        Permission::from_value(&PermissionResponse::AllowOnce.to_value())
            .unwrap()
            .to_value(),
        PermissionResponse::AllowOnce.to_value()
    );
    assert_eq!(Update::phenix_schema(), ExecutionUpdate::phenix_schema());
}
