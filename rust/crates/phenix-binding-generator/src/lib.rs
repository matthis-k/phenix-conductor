#![forbid(unsafe_code)]

//! Descriptor-driven source generation shared by language bindings.
//!
//! This crate reads only the fixed application descriptor. It never imports
//! runtime plugins, ACP adapters, or language-host integrations.

use phenix_application_interface::{generate, ApplicationDescriptor};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GenerationError {
    Descriptor(String),
}

impl std::fmt::Display for GenerationError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Descriptor(message) => {
                write!(formatter, "cannot generate binding API: {message}")
            }
        }
    }
}

impl std::error::Error for GenerationError {}

/// Generates a deterministic Lua descriptor module.
///
/// Native Lua ABI integration, async dispatch, and transport ownership remain
/// handwritten in the binding package. This output contains only stable
/// application identities and descriptor-owned type references.
pub fn lua(descriptor: &ApplicationDescriptor) -> Result<String, GenerationError> {
    generate::rust(descriptor).map_err(|error| GenerationError::Descriptor(error.to_string()))?;

    let mut source = String::new();
    source.push_str("-- Generated from the fixed Phenix application descriptor. Do not edit.\n");
    source.push_str("return {\n");
    line(
        &mut source,
        1,
        &format!("interface_id = {},", lua_string(descriptor.id.as_str())),
    );
    section(
        &mut source,
        "capabilities",
        descriptor.capabilities.keys().map(|id| id.as_str()),
    );
    source.push_str("  operations = {\n");
    for (id, operation) in &descriptor.operations {
        line(
            &mut source,
            2,
            &format!(
                "[{}] = {{ id = {}, capability = {}, input = {}, output = {}, error = {}, extension = {} }},",
                lua_string(id.as_str()),
                lua_string(id.as_str()),
                lua_string(operation.capability.as_str()),
                lua_string(operation.input.as_str()),
                lua_string(operation.output.as_str()),
                lua_string(operation.error.as_str()),
                lua_string(&extension_name(id.as_str())),
            ),
        );
    }
    source.push_str("  },\n");
    source.push_str("  events = {\n");
    for (id, event) in &descriptor.events {
        line(
            &mut source,
            2,
            &format!(
                "[{}] = {{ id = {}, capability = {}, payload = {}, ordering = {} }},",
                lua_string(id.as_str()),
                lua_string(id.as_str()),
                lua_string(event.capability.as_str()),
                lua_string(event.payload.as_str()),
                lua_string(&format!("{:?}", event.ordering).to_lowercase()),
            ),
        );
    }
    source.push_str("  },\n");
    source.push_str("  callbacks = {\n");
    for (id, callback) in &descriptor.callbacks {
        line(
            &mut source,
            2,
            &format!(
                "[{}] = {{ id = {}, capability = {}, request = {}, response = {}, semantics = {} }},",
                lua_string(id.as_str()),
                lua_string(id.as_str()),
                lua_string(callback.capability.as_str()),
                lua_string(callback.request.as_str()),
                lua_string(callback.response.as_str()),
                lua_string(&format!("{:?}", callback.semantics).to_lowercase()),
            ),
        );
    }
    source.push_str("  },\n");
    source.push_str("}\n");
    Ok(source)
}

fn section<'a>(source: &mut String, name: &str, ids: impl Iterator<Item = &'a str>) {
    line(source, 1, &format!("{name} = {{"));
    for id in ids {
        line(
            source,
            2,
            &format!("[{}] = {},", lua_string(id), lua_string(id)),
        );
    }
    line(source, 1, "},");
}

fn line(source: &mut String, indent: usize, value: &str) {
    source.push_str(&"  ".repeat(indent));
    source.push_str(value);
    source.push('\n');
}

fn extension_name(id: &str) -> String {
    let suffix = id.strip_prefix("phenix.application.").unwrap_or(id);
    format!("_phenix/{suffix}")
}

fn lua_string(value: &str) -> String {
    format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\""))
}

#[cfg(test)]
mod tests {
    use super::*;
    use phenix_application_interface::application_descriptor;

    #[test]
    fn lua_generation_is_deterministic_and_descriptor_owned() {
        let descriptor = application_descriptor();
        let first = lua(&descriptor).expect("fixed descriptor is Lua-generatable");
        let second = lua(&descriptor).expect("fixed descriptor is Lua-generatable");

        assert_eq!(first, second);
        assert!(first.contains("phenix.application@1"));
        assert!(first.contains("phenix.application.skill-list@1"));
        assert!(first.contains("_phenix/skill-list@1"));
        assert!(first.contains("phenix.application.capability.skills@1"));
    }
}
