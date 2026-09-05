use crate::wire;
use phenix_application_interface::types::{
    ApplicationError, PermissionRequest as ApplicationPermissionRequest,
    PermissionResponse as ApplicationPermissionResponse,
};
use wire::schema::v1::{
    PermissionOption, PermissionOptionId, PermissionOptionKind, RequestPermissionOutcome,
    RequestPermissionRequest, RequestPermissionResponse, ToolCallUpdate, ToolCallUpdateFields,
};

const ALLOW_ONCE: &str = "allow_once";
const DENY: &str = "deny";

#[must_use]
pub fn translate_permission_request(
    request: &ApplicationPermissionRequest,
) -> RequestPermissionRequest {
    let tool_call = ToolCallUpdate::new(
        request.call_id.clone(),
        ToolCallUpdateFields::new().title(request.description.clone()),
    );
    let mut meta = serde_json::Map::new();
    meta.insert(
        "phenix.executionId".to_owned(),
        serde_json::Value::String(request.execution_id.clone()),
    );
    RequestPermissionRequest::new(
        request.session_id.to_string(),
        tool_call,
        vec![
            PermissionOption::new(ALLOW_ONCE, "Allow once", PermissionOptionKind::AllowOnce),
            PermissionOption::new(DENY, "Deny", PermissionOptionKind::RejectOnce),
        ],
    )
    .meta(meta)
}

pub fn translate_permission_response(
    response: &RequestPermissionResponse,
) -> Result<ApplicationPermissionResponse, ApplicationError> {
    let allow_once = PermissionOptionId::new(ALLOW_ONCE);
    let deny = PermissionOptionId::new(DENY);
    match &response.outcome {
        RequestPermissionOutcome::Selected(selected) if selected.option_id == allow_once => {
            Ok(ApplicationPermissionResponse::AllowOnce)
        }
        RequestPermissionOutcome::Selected(selected) if selected.option_id == deny => {
            Ok(ApplicationPermissionResponse::Deny)
        }
        RequestPermissionOutcome::Selected(selected) => Err(ApplicationError::InvalidResponse {
            message: format!(
                "ACP permission response selected unknown option {}",
                selected.option_id
            ),
        }),
        RequestPermissionOutcome::Cancelled => Ok(ApplicationPermissionResponse::Cancelled),
        _ => Err(ApplicationError::InvalidResponse {
            message: "ACP permission response used an unsupported outcome".to_owned(),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use phenix_core::SessionId;
    use wire::schema::v1::SelectedPermissionOutcome;

    fn request() -> ApplicationPermissionRequest {
        ApplicationPermissionRequest {
            session_id: SessionId::parse("session-1").expect("valid session id"),
            execution_id: "execution-7".to_owned(),
            call_id: "call-1".to_owned(),
            description: "Write README.md".to_owned(),
        }
    }

    #[test]
    fn permission_request_uses_standard_acp_and_stable_execution_identity() {
        let translated = translate_permission_request(&request());
        let value = serde_json::to_value(translated).expect("permission request JSON");
        assert_eq!(value["sessionId"], "session-1");
        assert_eq!(value["toolCall"]["toolCallId"], "call-1");
        assert_eq!(value["toolCall"]["title"], "Write README.md");
        assert_eq!(value["_meta"]["phenix.executionId"], "execution-7");
        assert_eq!(value["options"][0]["optionId"], ALLOW_ONCE);
        assert_eq!(value["options"][1]["optionId"], DENY);
    }

    #[test]
    fn permission_response_maps_only_declared_choices() {
        let allow = RequestPermissionResponse::new(RequestPermissionOutcome::Selected(
            SelectedPermissionOutcome::new(ALLOW_ONCE),
        ));
        assert_eq!(
            translate_permission_response(&allow).expect("allow response"),
            ApplicationPermissionResponse::AllowOnce
        );

        let deny = RequestPermissionResponse::new(RequestPermissionOutcome::Selected(
            SelectedPermissionOutcome::new(DENY),
        ));
        assert_eq!(
            translate_permission_response(&deny).expect("deny response"),
            ApplicationPermissionResponse::Deny
        );

        let cancelled = RequestPermissionResponse::new(RequestPermissionOutcome::Cancelled);
        assert_eq!(
            translate_permission_response(&cancelled).expect("cancelled response"),
            ApplicationPermissionResponse::Cancelled
        );

        let unknown = RequestPermissionResponse::new(RequestPermissionOutcome::Selected(
            SelectedPermissionOutcome::new("allow_always"),
        ));
        assert!(matches!(
            translate_permission_response(&unknown),
            Err(ApplicationError::InvalidResponse { .. })
        ));
    }
}
