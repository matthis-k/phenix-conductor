use std::fmt::{self, Display, Formatter};

macro_rules! identifier {
    ($name:ident) => {
        #[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
        pub struct $name(String);

        impl $name {
            pub fn parse(value: impl Into<String>) -> Result<Self, &'static str> {
                let value = value.into();
                if value.is_empty() {
                    return Err("identifier must not be empty");
                }
                if !value.bytes().all(|byte| {
                    byte.is_ascii_alphanumeric()
                        || matches!(byte, b'.' | b'_' | b'-' | b'/' | b':' | b'@')
                }) {
                    return Err("identifier contains unsupported characters");
                }
                Ok(Self(value))
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl Display for $name {
            fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
                f.write_str(&self.0)
            }
        }
    };
}

identifier!(PluginId);
identifier!(ServiceId);
identifier!(CapabilityId);
identifier!(ResourceNamespace);
identifier!(EventTypeId);
identifier!(SubscriptionId);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identifiers_are_stable_and_reject_ambiguous_text() {
        let id = ServiceId::parse("artifact.read@1").unwrap();
        assert_eq!(id.as_str(), "artifact.read@1");
        assert!(PluginId::parse("").is_err());
        assert!(PluginId::parse("has space").is_err());
    }
}
