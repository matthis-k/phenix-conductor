use phenix_adapter_acp::{wire, ApplicationAdapter};
use phenix_application_interface::{
    application_descriptor,
    types::{
        Acknowledged, ApplicationError, Content, ModelInfo, ModelSelectInput, Models, PageInput,
        PromptInput, PromptResult, RoutingInfo, RoutingProfiles, RoutingSelectInput, SessionInfo,
        SessionList, SessionResumeInput, SessionSnapshot, StopReason,
    },
    ApplicationTransport, Cancel, CloseSession, CreateSession, ListModels, ListRoutingProfiles,
    ListSessions, Operation, Prompt, ResumeSession, SelectModel, SelectRoutingProfile,
};
use phenix_core::{ContractId, ModelId, PhenixValue, RoutingProfileId, SessionId, ValueCodec};
use std::sync::{Arc, Mutex};
use wire::schema::v1::{
    ContentBlock, InitializeRequest, NewSessionRequest, PromptRequest, ResourceLink,
    SetSessionConfigOptionRequest, TextContent,
};
use wire::schema::ProtocolVersion;

#[derive(Clone)]
struct FakeTransport {
    calls: Arc<Mutex<Vec<(String, PhenixValue)>>>,
    selected_model: Arc<Mutex<String>>,
    selected_routing: Arc<Mutex<String>>,
}

impl Default for FakeTransport {
    fn default() -> Self {
        Self {
            calls: Arc::new(Mutex::new(Vec::new())),
            selected_model: Arc::new(Mutex::new("model-a".to_owned())),
            selected_routing: Arc::new(Mutex::new("balanced".to_owned())),
        }
    }
}

impl ApplicationTransport for FakeTransport {
    fn invoke(
        &self,
        operation: &ContractId,
        input: PhenixValue,
    ) -> impl std::future::Future<Output = Result<PhenixValue, ApplicationError>> {
        let calls = self.calls.clone();
        let selected_model = self.selected_model.clone();
        let selected_routing = self.selected_routing.clone();
        let operation = operation.as_str().to_owned();
        async move {
            calls
                .lock()
                .expect("fake transport calls lock")
                .push((operation.clone(), input.clone()));
            match operation.as_str() {
                id if id == CreateSession::ID => Ok(session_info().to_value()),
                id if id == ListSessions::ID => Ok(SessionList {
                    sessions: vec![session_info()],
                    next_cursor: Some("next".to_owned()),
                }
                .to_value()),
                id if id == ResumeSession::ID => Ok(SessionSnapshot {
                    session: session_info(),
                    through_sequence: 9,
                    updates: Vec::new(),
                }
                .to_value()),
                id if id == CloseSession::ID || id == Cancel::ID => Ok(Acknowledged {}.to_value()),
                id if id == Prompt::ID => Ok(PromptResult {
                    execution_id: "execution-7".to_owned(),
                    stop_reason: StopReason::EndTurn,
                }
                .to_value()),
                id if id == ListModels::ID => {
                    let selected = selected_model.lock().expect("selected model lock").clone();
                    Ok(models(&selected).to_value())
                }
                id if id == SelectModel::ID => {
                    let selection =
                        ModelSelectInput::from_value(&input).expect("typed model selection");
                    let selected = selection.model_id.to_string();
                    selected_model
                        .lock()
                        .expect("selected model lock")
                        .clone_from(&selected);
                    Ok(models(&selected).to_value())
                }
                id if id == ListRoutingProfiles::ID => {
                    let selected = selected_routing
                        .lock()
                        .expect("selected routing lock")
                        .clone();
                    Ok(routing_profiles(&selected).to_value())
                }
                id if id == SelectRoutingProfile::ID => {
                    let selection =
                        RoutingSelectInput::from_value(&input).expect("typed routing selection");
                    let selected = selection.profile_id.to_string();
                    selected_routing
                        .lock()
                        .expect("selected routing lock")
                        .clone_from(&selected);
                    Ok(routing_profiles(&selected).to_value())
                }
                other => Err(ApplicationError::Failed {
                    message: format!("unexpected operation {other}"),
                }),
            }
        }
    }
}

fn session_info() -> SessionInfo {
    SessionInfo {
        session_id: SessionId::parse("session-1").expect("valid session id"),
        title: Some("Example".to_owned()),
        working_directory: "/workspace".to_owned(),
    }
}

fn models(selected: &str) -> Models {
    Models {
        available: vec![
            ModelInfo {
                id: ModelId::parse("model-a").expect("valid model id"),
                name: "Model A".to_owned(),
                description: Some("Fast model".to_owned()),
            },
            ModelInfo {
                id: ModelId::parse("model-b").expect("valid model id"),
                name: "Model B".to_owned(),
                description: Some("Deep model".to_owned()),
            },
        ],
        selected: Some(ModelId::parse(selected).expect("valid selected model")),
    }
}

fn routing_profiles(selected: &str) -> RoutingProfiles {
    RoutingProfiles {
        available: vec![
            RoutingInfo {
                id: RoutingProfileId::parse("balanced").expect("valid routing id"),
                name: "Balanced".to_owned(),
            },
            RoutingInfo {
                id: RoutingProfileId::parse("deep").expect("valid routing id"),
                name: "Deep".to_owned(),
            },
        ],
        selected: Some(RoutingProfileId::parse(selected).expect("valid selected routing")),
    }
}

fn adapter() -> (ApplicationAdapter<FakeTransport>, FakeTransport) {
    let transport = FakeTransport::default();
    let descriptor = application_descriptor();
    let advertised = descriptor.capabilities.keys().cloned();
    let adapter =
        ApplicationAdapter::new(transport.clone(), advertised).expect("full capabilities");
    (adapter, transport)
}

#[test]
fn initialize_advertises_only_implemented_standard_and_descriptor_extensions() {
    let (adapter, _) = adapter();
    let response = adapter.initialize(InitializeRequest::new(ProtocolVersion::V1));
    let value = serde_json::to_value(response).expect("serialize initialize response");

    assert_eq!(value["agentCapabilities"]["loadSession"], true);
    assert!(value["agentCapabilities"]["sessionCapabilities"]["list"].is_object());
    assert!(value["agentCapabilities"]["sessionCapabilities"]["resume"].is_object());
    assert!(value["agentCapabilities"]["sessionCapabilities"]["close"].is_object());

    let extensions = &value["_meta"]["phenix.extensions"];
    assert_eq!(extensions["interface"], "phenix.application@1");
    let methods = extensions["methods"].as_array().expect("extension methods");
    let skill_list = methods
        .iter()
        .find(|method| method["method"] == "_phenix/skill-list@1")
        .expect("skill list extension");
    assert_eq!(skill_list["operation"], "phenix.application.skill-list@1");
    assert_eq!(
        skill_list["capability"],
        "phenix.application.capability.skills@1"
    );
    assert!(skill_list["input"].is_object());
    assert!(skill_list["output"].is_object());

    for mapped in [
        "_phenix/model-list@1",
        "_phenix/model-select@1",
        "_phenix/routing-list@1",
        "_phenix/routing-select@1",
    ] {
        assert!(methods.iter().all(|method| method["method"] != mapped));
    }
    assert!(methods
        .iter()
        .any(|method| method["method"] == "_phenix/authentication-list@1"));

    for lane in ["methods", "events", "callbacks"] {
        for extension in extensions[lane]
            .as_array()
            .unwrap_or_else(|| panic!("{lane} extension lane"))
        {
            let method = extension["method"].as_str().expect("extension method name");
            assert!(method.starts_with("_phenix/"));
            assert!(!method.contains("client/envelope"));
        }
    }
}

#[tokio::test]
async fn standard_session_and_prompt_requests_use_typed_application_operations() {
    let (adapter, transport) = adapter();

    let created = adapter
        .new_session(NewSessionRequest::new("/workspace"))
        .await
        .expect("create session");
    assert_eq!(created.session_id.to_string(), "session-1");
    assert_eq!(created.config_options.as_ref().map(Vec::len), Some(2));

    let prompt = PromptRequest::new(
        "session-1",
        vec![
            ContentBlock::Text(TextContent::new("hello")),
            ContentBlock::ResourceLink(ResourceLink::new("README", "file:///workspace/README.md")),
        ],
    );
    let response = adapter.prompt(prompt).await.expect("prompt");
    assert_eq!(response.stop_reason, wire::schema::v1::StopReason::EndTurn);
    assert_eq!(
        response.meta.expect("prompt meta")["phenix.executionId"],
        "execution-7"
    );

    let calls = transport.calls.lock().expect("calls lock");
    assert_eq!(calls[0].0, CreateSession::ID);
    let created_input =
        phenix_application_interface::types::SessionCreateInput::from_value(&calls[0].1)
            .expect("typed create input");
    assert_eq!(created_input.working_directory, "/workspace");

    let prompt_call = calls
        .iter()
        .find(|(operation, _)| operation == Prompt::ID)
        .expect("prompt call");
    let prompt_input = PromptInput::from_value(&prompt_call.1).expect("typed prompt input");
    assert_eq!(prompt_input.session_id.as_str(), "session-1");
    assert_eq!(
        prompt_input.content,
        vec![
            Content::Text {
                text: "hello".to_owned()
            },
            Content::Resource {
                uri: "file:///workspace/README.md".to_owned(),
                mime_type: None,
                text: None,
            },
        ]
    );
}

#[tokio::test]
async fn model_and_routing_state_use_standard_acp_config_options() {
    let (adapter, transport) = adapter();
    let created = adapter
        .new_session(NewSessionRequest::new("/workspace"))
        .await
        .expect("create session");
    let created = serde_json::to_value(created).expect("new session JSON");
    let options = created["configOptions"]
        .as_array()
        .expect("initial config options");
    let model = options
        .iter()
        .find(|option| option["id"] == "model")
        .expect("model config");
    assert_eq!(model["category"], "model");
    assert_eq!(model["currentValue"], "model-a");
    let routing = options
        .iter()
        .find(|option| option["id"] == "_phenix/routing-profile")
        .expect("routing config");
    assert_eq!(routing["category"], "_phenix/routing");
    assert_eq!(routing["currentValue"], "balanced");

    let updated = adapter
        .set_session_config_option(SetSessionConfigOptionRequest::new(
            "session-1",
            "model",
            "model-b",
        ))
        .await
        .expect("select model");
    let updated = serde_json::to_value(updated).expect("config response JSON");
    let model = updated["configOptions"]
        .as_array()
        .expect("updated config options")
        .iter()
        .find(|option| option["id"] == "model")
        .expect("updated model config");
    assert_eq!(model["currentValue"], "model-b");

    let updated = adapter
        .set_session_config_option(SetSessionConfigOptionRequest::new(
            "session-1",
            "_phenix/routing-profile",
            "deep",
        ))
        .await
        .expect("select routing profile");
    let updated = serde_json::to_value(updated).expect("routing response JSON");
    let routing = updated["configOptions"]
        .as_array()
        .expect("updated config options")
        .iter()
        .find(|option| option["id"] == "_phenix/routing-profile")
        .expect("updated routing config");
    assert_eq!(routing["currentValue"], "deep");

    let calls = transport.calls.lock().expect("calls lock");
    let model_call = calls
        .iter()
        .find(|(operation, _)| operation == SelectModel::ID)
        .expect("model selection call");
    let model_input = ModelSelectInput::from_value(&model_call.1).expect("model selection input");
    assert_eq!(model_input.session_id.as_str(), "session-1");
    assert_eq!(model_input.model_id.as_str(), "model-b");

    let routing_call = calls
        .iter()
        .find(|(operation, _)| operation == SelectRoutingProfile::ID)
        .expect("routing selection call");
    let routing_input =
        RoutingSelectInput::from_value(&routing_call.1).expect("routing selection input");
    assert_eq!(routing_input.session_id.as_str(), "session-1");
    assert_eq!(routing_input.profile_id.as_str(), "deep");
}

#[tokio::test]
async fn unsupported_security_relevant_session_inputs_fail_before_runtime_dispatch() {
    let (adapter, transport) = adapter();
    let error = adapter
        .new_session(
            NewSessionRequest::new("/workspace").additional_directories(vec!["/outside".into()]),
        )
        .await
        .expect_err("additional directory must fail");
    assert!(matches!(error, ApplicationError::InvalidInput { .. }));
    assert!(transport.calls.lock().expect("calls lock").is_empty());
}

#[tokio::test]
async fn list_and_resume_preserve_durable_session_identity_and_cwd() {
    let (adapter, transport) = adapter();
    let listed = adapter
        .list_sessions(wire::schema::v1::ListSessionsRequest::new().cwd("/workspace"))
        .await
        .expect("list sessions");
    assert_eq!(listed.sessions.len(), 1);
    assert_eq!(listed.sessions[0].session_id.to_string(), "session-1");
    assert_eq!(listed.next_cursor.as_deref(), Some("next"));

    let resumed = adapter
        .resume_session(wire::schema::v1::ResumeSessionRequest::new(
            "session-1",
            "/workspace",
        ))
        .await
        .expect("resume session");
    assert_eq!(resumed.config_options.as_ref().map(Vec::len), Some(2));

    let calls = transport.calls.lock().expect("calls lock");
    assert_eq!(calls[0].0, ListSessions::ID);
    let page = PageInput::from_value(&calls[0].1).expect("typed page input");
    assert_eq!(page.cursor, None);
    assert_eq!(calls[1].0, ResumeSession::ID);
    let resume = SessionResumeInput::from_value(&calls[1].1).expect("typed resume input");
    assert_eq!(resume.session_id.as_str(), "session-1");
    assert_eq!(resume.after_sequence, None);
}
