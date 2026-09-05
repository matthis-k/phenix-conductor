use crate::wire;
use phenix_application_interface::types::{
    ApplicationError, ElicitationRequest as ApplicationElicitationRequest,
    ElicitationResponse as ApplicationElicitationResponse,
};
use phenix_core::{PhenixSchema, PhenixValue, Type};
use wire::schema::v1::{
    ClientCapabilities, CreateElicitationRequest, CreateElicitationResponse, ElicitationAction,
    ElicitationContentValue, ElicitationFormMode, ElicitationSchema, ElicitationSessionScope,
};

#[must_use]
pub fn translate_elicitation_request(
    request: &ApplicationElicitationRequest,
    client_capabilities: &ClientCapabilities,
) -> Option<CreateElicitationRequest> {
    if client_capabilities
        .elicitation
        .as_ref()
        .is_none_or(|elicitation| elicitation.form.is_none())
    {
        return None;
    }
    let schema = standard_elicitation_schema(&request.schema)?;
    Some(CreateElicitationRequest::new(
        ElicitationFormMode::new(
            ElicitationSessionScope::new(request.session_id.to_string()),
            schema,
        ),
        request.message.clone(),
    ))
}

pub fn translate_elicitation_response(
    schema: &PhenixSchema,
    response: &CreateElicitationResponse,
) -> Result<ApplicationElicitationResponse, ApplicationError> {
    let Type::Table(fields) = schema else {
        return Err(unsupported_schema());
    };
    if standard_elicitation_schema(schema).is_none() {
        return Err(unsupported_schema());
    }

    match &response.action {
        ElicitationAction::Accept(accepted) => {
            let empty = std::collections::BTreeMap::new();
            let content = accepted.content.as_ref().unwrap_or(&empty);
            if let Some(key) = content
                .keys()
                .find(|key| !fields.contains_key(key.as_str()))
            {
                return Err(ApplicationError::InvalidResponse {
                    message: format!("ACP elicitation returned unexpected field {key}"),
                });
            }

            let value = PhenixValue::Table(
                fields
                    .iter()
                    .map(|(key, expected)| {
                        elicitation_value(expected, content.get(key.as_str()))
                            .map(|value| (key.clone(), value))
                    })
                    .collect::<Result<_, _>>()?,
            );
            schema
                .parse(&value)
                .map_err(|error| ApplicationError::InvalidResponse {
                    message: format!(
                        "ACP elicitation response violates the Phenix schema: {error}"
                    ),
                })?;
            Ok(ApplicationElicitationResponse::Accepted { value })
        }
        ElicitationAction::Decline => Ok(ApplicationElicitationResponse::Declined),
        ElicitationAction::Cancel => Ok(ApplicationElicitationResponse::Cancelled),
        _ => Err(ApplicationError::InvalidResponse {
            message: "ACP elicitation response used an unsupported action".to_owned(),
        }),
    }
}

fn standard_elicitation_schema(schema: &PhenixSchema) -> Option<ElicitationSchema> {
    let Type::Table(fields) = schema else {
        return None;
    };
    let mut result = ElicitationSchema::new();
    for (key, field) in fields {
        let (field, required) = match field {
            Type::Option(inner) => (inner.as_ref(), false),
            other => (other, true),
        };
        result = match field {
            Type::String => result.string(key.as_str(), required),
            Type::Bool => result.boolean(key.as_str(), required),
            Type::I64 => result.integer(key.as_str(), i64::MIN, i64::MAX, required),
            Type::F64 => result.number(key.as_str(), f64::MIN, f64::MAX, required),
            _ => return None,
        };
    }
    Some(result)
}

fn elicitation_value(
    expected: &Type,
    value: Option<&ElicitationContentValue>,
) -> Result<PhenixValue, ApplicationError> {
    if let Type::Option(inner) = expected {
        return value
            .map(|value| {
                elicitation_value(inner, Some(value))
                    .map(|value| PhenixValue::Option(Some(Box::new(value))))
            })
            .unwrap_or_else(|| Ok(PhenixValue::Option(None)));
    }

    match (expected, value) {
        (Type::String, Some(ElicitationContentValue::String(value))) => {
            Ok(PhenixValue::String(value.clone()))
        }
        (Type::Bool, Some(ElicitationContentValue::Boolean(value))) => {
            Ok(PhenixValue::Bool(*value))
        }
        (Type::I64, Some(ElicitationContentValue::Integer(value))) => Ok(PhenixValue::I64(*value)),
        (Type::F64, Some(ElicitationContentValue::Number(value))) => Ok(PhenixValue::F64(*value)),
        (_, None) => Err(ApplicationError::InvalidResponse {
            message: "ACP elicitation response omitted a required field".to_owned(),
        }),
        _ => Err(ApplicationError::InvalidResponse {
            message: "ACP elicitation response field has the wrong primitive type".to_owned(),
        }),
    }
}

fn unsupported_schema() -> ApplicationError {
    ApplicationError::InvalidResponse {
        message: "ACP elicitation response has no matching standard Phenix form schema".to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use phenix_core::{Key, SessionId};
    use std::collections::BTreeMap;
    use wire::schema::v1::{
        ElicitationAcceptAction, ElicitationCapabilities, ElicitationFormCapabilities,
    };

    fn request(schema: PhenixSchema) -> ApplicationElicitationRequest {
        ApplicationElicitationRequest {
            session_id: SessionId::parse("session-1").expect("valid session id"),
            message: "Configure the run".to_owned(),
            schema,
        }
    }

    fn form_client() -> ClientCapabilities {
        ClientCapabilities::new()
            .elicitation(ElicitationCapabilities::new().form(ElicitationFormCapabilities::new()))
    }

    #[test]
    fn primitive_table_uses_standard_acp_form() {
        let schema = Type::Table(BTreeMap::from([
            (Key::parse("name").unwrap(), Type::String),
            (
                Key::parse("confirmed").unwrap(),
                Type::Option(Box::new(Type::Bool)),
            ),
            (Key::parse("count").unwrap(), Type::I64),
        ]));
        let translated = translate_elicitation_request(&request(schema), &form_client())
            .expect("standard form mapping");
        let value = serde_json::to_value(translated).expect("elicitation request JSON");
        assert_eq!(value["mode"], "form");
        assert_eq!(value["sessionId"], "session-1");
        assert_eq!(
            value["requestedSchema"]["properties"]["name"]["type"],
            "string"
        );
        assert_eq!(
            value["requestedSchema"]["properties"]["confirmed"]["type"],
            "boolean"
        );
        let required = value["requestedSchema"]["required"]
            .as_array()
            .expect("required fields");
        assert!(required.contains(&serde_json::Value::String("name".to_owned())));
        assert!(required.contains(&serde_json::Value::String("count".to_owned())));
        assert!(!required.contains(&serde_json::Value::String("confirmed".to_owned())));
    }

    #[test]
    fn standard_form_requires_client_support_and_supported_schema() {
        let supported = Type::Table(BTreeMap::from([(
            Key::parse("name").unwrap(),
            Type::String,
        )]));
        assert!(
            translate_elicitation_request(&request(supported), &ClientCapabilities::new())
                .is_none()
        );

        let unsupported = Type::Table(BTreeMap::from([(
            Key::parse("tags").unwrap(),
            Type::List(Box::new(Type::String)),
        )]));
        assert!(translate_elicitation_request(&request(unsupported), &form_client()).is_none());
    }

    #[test]
    fn response_restores_optional_values_and_rejects_extra_fields() {
        let schema = Type::Table(BTreeMap::from([
            (Key::parse("name").unwrap(), Type::String),
            (
                Key::parse("confirmed").unwrap(),
                Type::Option(Box::new(Type::Bool)),
            ),
        ]));
        let accepted = CreateElicitationResponse::new(ElicitationAction::Accept(
            ElicitationAcceptAction::new().content(BTreeMap::from([(
                "name".to_owned(),
                ElicitationContentValue::from("Ada"),
            )])),
        ));
        assert_eq!(
            translate_elicitation_response(&schema, &accepted).expect("accepted response"),
            ApplicationElicitationResponse::Accepted {
                value: PhenixValue::Table(BTreeMap::from([
                    (Key::parse("confirmed").unwrap(), PhenixValue::Option(None)),
                    (
                        Key::parse("name").unwrap(),
                        PhenixValue::String("Ada".to_owned()),
                    ),
                ])),
            }
        );

        let extra = CreateElicitationResponse::new(ElicitationAction::Accept(
            ElicitationAcceptAction::new().content(BTreeMap::from([
                ("name".to_owned(), ElicitationContentValue::from("Ada")),
                ("extra".to_owned(), ElicitationContentValue::from(true)),
            ])),
        ));
        assert!(matches!(
            translate_elicitation_response(&schema, &extra),
            Err(ApplicationError::InvalidResponse { .. })
        ));
    }
}
