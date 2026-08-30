use crate::{Endpoint, ProviderError, ProviderRequest, ProviderResponse, RateLimits};
use phenix_core::{ModelInferenceRequest, ModelInferenceResponse};
use reqwest::header::CONTENT_TYPE;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::BTreeMap;

pub trait ProtocolAdapter: Send + Sync {
    fn name(&self) -> &'static str;

    fn encode(
        &self,
        endpoint: &Endpoint,
        request: &ModelInferenceRequest,
    ) -> Result<ProviderRequest, ProviderError>;

    fn decode(&self, response: &ProviderResponse) -> Result<ModelInferenceResponse, ProviderError>;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Protocol {
    OpenAiResponses,
    OpenAiChatCompletions,
    AnthropicMessages,
}

impl ProtocolAdapter for Protocol {
    fn name(&self) -> &'static str {
        match self {
            Self::OpenAiResponses => "openai_responses",
            Self::OpenAiChatCompletions => "openai_chat_completions",
            Self::AnthropicMessages => "anthropic_messages",
        }
    }

    fn encode(
        &self,
        endpoint: &Endpoint,
        request: &ModelInferenceRequest,
    ) -> Result<ProviderRequest, ProviderError> {
        match self {
            Self::OpenAiResponses => openai_responses_request(endpoint, request),
            Self::OpenAiChatCompletions => openai_chat_request(endpoint, request),
            Self::AnthropicMessages => anthropic_request(endpoint, request),
        }
    }

    fn decode(&self, response: &ProviderResponse) -> Result<ModelInferenceResponse, ProviderError> {
        match self {
            Self::OpenAiResponses => openai_responses_response(response),
            Self::OpenAiChatCompletions => openai_chat_response(response),
            Self::AnthropicMessages => anthropic_response(response),
        }
    }
}

fn base_request(
    endpoint: &Endpoint,
    path: &str,
    body: Value,
    headers: BTreeMap<String, String>,
) -> Result<ProviderRequest, ProviderError> {
    Ok(ProviderRequest {
        method: crate::HttpMethod::Post,
        url: endpoint.join(path)?,
        headers,
        body: serde_json::to_vec(&body).map_err(|error| ProviderError::Protocol {
            message: error.to_string(),
        })?,
    })
}

fn json_headers() -> BTreeMap<String, String> {
    BTreeMap::from([(
        CONTENT_TYPE.as_str().to_owned(),
        "application/json".to_owned(),
    )])
}

fn request_object(
    request: &ModelInferenceRequest,
    reserved: &[&str],
) -> Result<(Map<String, Value>, String), ProviderError> {
    let text = std::str::from_utf8(request.input.as_ref())
        .map_err(|_| ProviderError::InvalidRequest {
            message: "provider protocols require UTF-8 model input".to_owned(),
        })?
        .to_owned();
    let mut body = Map::new();
    for (key, value) in &request.options {
        if reserved.contains(&key.as_str()) {
            return Err(ProviderError::InvalidRequest {
                message: format!(
                    "provider option {key:?} conflicts with a required protocol field"
                ),
            });
        }
        body.insert(key.clone(), value.clone());
    }
    Ok((body, text))
}

fn openai_responses_request(
    endpoint: &Endpoint,
    request: &ModelInferenceRequest,
) -> Result<ProviderRequest, ProviderError> {
    let (mut body, text) = request_object(request, &["model", "input"])?;
    body.insert(
        "model".to_owned(),
        Value::String(request.model.as_str().to_owned()),
    );
    body.insert("input".to_owned(), Value::String(text));
    base_request(endpoint, "responses", Value::Object(body), json_headers())
}

fn openai_chat_request(
    endpoint: &Endpoint,
    request: &ModelInferenceRequest,
) -> Result<ProviderRequest, ProviderError> {
    let (mut body, text) = request_object(request, &["model", "messages"])?;
    body.insert(
        "model".to_owned(),
        Value::String(request.model.as_str().to_owned()),
    );
    body.insert(
        "messages".to_owned(),
        serde_json::json!([{"role":"user","content":text}]),
    );
    base_request(
        endpoint,
        "chat/completions",
        Value::Object(body),
        json_headers(),
    )
}

fn anthropic_request(
    endpoint: &Endpoint,
    request: &ModelInferenceRequest,
) -> Result<ProviderRequest, ProviderError> {
    let (mut body, text) = request_object(request, &["model", "messages"])?;
    body.insert(
        "model".to_owned(),
        Value::String(request.model.as_str().to_owned()),
    );
    body.insert(
        "messages".to_owned(),
        serde_json::json!([{"role":"user","content":text}]),
    );
    body.entry("max_tokens".to_owned())
        .or_insert_with(|| Value::from(4096_u64));
    let mut headers = json_headers();
    headers.insert("anthropic-version".to_owned(), "2023-06-01".to_owned());
    base_request(endpoint, "messages", Value::Object(body), headers)
}

fn parse_json(response: &ProviderResponse) -> Result<Value, ProviderError> {
    serde_json::from_slice(&response.body).map_err(|error| ProviderError::Protocol {
        message: format!("cannot parse provider JSON response: {error}"),
    })
}

fn response_with_text(value: &Value, text: String) -> ModelInferenceResponse {
    let mut provider_metadata = BTreeMap::new();
    if let Some(id) = value.get("id").cloned() {
        provider_metadata.insert("id".to_owned(), id);
    }
    if let Some(usage) = value.get("usage").cloned() {
        provider_metadata.insert("usage".to_owned(), usage);
    }
    ModelInferenceResponse {
        output: text.into_bytes().into(),
        provider_metadata,
    }
}

fn openai_responses_response(
    response: &ProviderResponse,
) -> Result<ModelInferenceResponse, ProviderError> {
    let value = parse_json(response)?;
    if let Some(text) = value.get("output_text").and_then(Value::as_str) {
        return Ok(response_with_text(&value, text.to_owned()));
    }
    let text = value
        .get("output")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|item| item.get("content").and_then(Value::as_array))
        .flatten()
        .filter_map(|part| {
            part.get("text")
                .and_then(Value::as_str)
                .or_else(|| part.get("output_text").and_then(Value::as_str))
        })
        .collect::<Vec<_>>()
        .join("");
    if text.is_empty() {
        return Err(ProviderError::Protocol {
            message: "OpenAI responses payload contained no output text".to_owned(),
        });
    }
    Ok(response_with_text(&value, text))
}

fn openai_chat_response(
    response: &ProviderResponse,
) -> Result<ModelInferenceResponse, ProviderError> {
    let value = parse_json(response)?;
    let content =
        value
            .pointer("/choices/0/message/content")
            .ok_or_else(|| ProviderError::Protocol {
                message: "OpenAI chat payload contained no first choice content".to_owned(),
            })?;
    let text = if let Some(text) = content.as_str() {
        text.to_owned()
    } else {
        content
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|part| part.get("text").and_then(Value::as_str))
            .collect::<Vec<_>>()
            .join("")
    };
    if text.is_empty() {
        return Err(ProviderError::Protocol {
            message: "OpenAI chat payload contained empty content".to_owned(),
        });
    }
    Ok(response_with_text(&value, text))
}

fn anthropic_response(
    response: &ProviderResponse,
) -> Result<ModelInferenceResponse, ProviderError> {
    let value = parse_json(response)?;
    let text = value
        .get("content")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|part| part.get("text").and_then(Value::as_str))
        .collect::<Vec<_>>()
        .join("");
    if text.is_empty() {
        return Err(ProviderError::Protocol {
            message: "Anthropic messages payload contained no text content".to_owned(),
        });
    }
    Ok(response_with_text(&value, text))
}

pub fn normalize_http_error(response: &ProviderResponse) -> ProviderError {
    let message = error_message(&response.body);
    let normalized = message.to_ascii_lowercase();
    if response.status == 413
        || [
            "context_length_exceeded",
            "maximum context length",
            "context window",
            "too many tokens",
            "input is too long",
            "prompt is too long",
        ]
        .iter()
        .any(|needle| normalized.contains(needle))
    {
        return ProviderError::ContextLimit { message };
    }
    match response.status {
        400 | 409 | 422 => ProviderError::InvalidRequest { message },
        401 => ProviderError::Authentication { message },
        403 => ProviderError::Permission { message },
        404 => ProviderError::NotFound { message },
        408 | 425 | 500..=599 => ProviderError::Unavailable { message },
        429 => ProviderError::RateLimited {
            message,
            limits: Box::new(RateLimits::from_headers(&response.headers)),
        },
        _ => ProviderError::Protocol {
            message: format!("HTTP {}: {message}", response.status),
        },
    }
}

fn error_message(body: &[u8]) -> String {
    let Ok(value) = serde_json::from_slice::<Value>(body) else {
        let text = String::from_utf8_lossy(body).trim().to_owned();
        return if text.is_empty() {
            "provider request failed".to_owned()
        } else {
            text
        };
    };
    value
        .pointer("/error/message")
        .and_then(Value::as_str)
        .or_else(|| value.get("message").and_then(Value::as_str))
        .or_else(|| value.pointer("/error/code").and_then(Value::as_str))
        .or_else(|| value.get("error").and_then(Value::as_str))
        .unwrap_or("provider request failed")
        .to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DurationMs, ProviderResponse};
    use std::collections::BTreeMap;

    fn request() -> ModelInferenceRequest {
        ModelInferenceRequest {
            model: phenix_core::ModelId::parse("test-model").unwrap(),
            input: b"hello".to_vec().into(),
            options: BTreeMap::new(),
        }
    }

    fn response(status: u16, headers: &[(&str, &str)], body: Value) -> ProviderResponse {
        ProviderResponse {
            status,
            headers: headers
                .iter()
                .map(|(key, value)| ((*key).to_owned(), (*value).to_owned()))
                .collect(),
            body: serde_json::to_vec(&body).unwrap(),
        }
    }

    #[test]
    fn openai_responses_maps_internal_request_and_response() {
        let endpoint = Endpoint::parse("https://example.com/v1").unwrap();
        let encoded = Protocol::OpenAiResponses
            .encode(&endpoint, &request())
            .unwrap();
        assert_eq!(encoded.url, "https://example.com/v1/responses");
        let body: Value = serde_json::from_slice(&encoded.body).unwrap();
        assert_eq!(body["model"], "test-model");
        assert_eq!(body["input"], "hello");

        let decoded = Protocol::OpenAiResponses
            .decode(&response(
                200,
                &[],
                serde_json::json!({
                    "id":"response-1",
                    "output":[{"content":[{"type":"output_text","text":"world"}]}],
                    "usage":{"input_tokens":1,"output_tokens":1}
                }),
            ))
            .unwrap();
        assert_eq!(decoded.output.as_ref(), b"world");
        assert_eq!(decoded.provider_metadata["id"], "response-1");
    }

    #[test]
    fn anthropic_messages_maps_internal_request_and_response() {
        let endpoint = Endpoint::parse("https://example.com/v1").unwrap();
        let encoded = Protocol::AnthropicMessages
            .encode(&endpoint, &request())
            .unwrap();
        assert_eq!(encoded.url, "https://example.com/v1/messages");
        let body: Value = serde_json::from_slice(&encoded.body).unwrap();
        assert_eq!(body["max_tokens"], 4096);

        let decoded = Protocol::AnthropicMessages
            .decode(&response(
                200,
                &[],
                serde_json::json!({
                    "content":[{"type":"text","text":"world"}]
                }),
            ))
            .unwrap();
        assert_eq!(decoded.output.as_ref(), b"world");
    }

    #[test]
    fn common_http_failures_are_normalized() {
        let error = normalize_http_error(&response(
            429,
            &[("retry-after", "3")],
            serde_json::json!({"error":{"message":"rate limit"}}),
        ));
        let ProviderError::RateLimited { limits, .. } = error else {
            panic!("expected rate-limited error");
        };
        assert_eq!(limits.retry_after, Some(DurationMs(3000)));

        let error = normalize_http_error(&response(
            400,
            &[],
            serde_json::json!({
                "error":{
                    "code":"context_length_exceeded",
                    "message":"too many tokens"
                }
            }),
        ));
        assert!(matches!(error, ProviderError::ContextLimit { .. }));
    }
}
