use serde::{de::Error as _, Deserialize, Deserializer, Serialize, Serializer};
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

        impl Serialize for $name {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: Serializer,
            {
                serializer.serialize_str(self.as_str())
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: Deserializer<'de>,
            {
                let value = String::deserialize(deserializer)?;
                Self::parse(value).map_err(D::Error::custom)
            }
        }
    };
}

identifier!(PluginId);
identifier!(ComponentId);
identifier!(ConfigurationFrontendId);
identifier!(ServiceId);
identifier!(CapabilityId);
identifier!(ResourceNamespace);
identifier!(EventTypeId);
identifier!(SubscriptionId);

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct InterfaceId(String);

impl InterfaceId {
    pub fn parse(value: impl Into<String>) -> Result<Self, &'static str> {
        let value = value.into();
        if value.is_empty() {
            return Err("interface identifier must not be empty");
        }
        if !value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-' | b'/' | b':' | b'@')
        }) {
            return Err("interface identifier contains unsupported characters");
        }
        let (identity, version) = value
            .rsplit_once('@')
            .ok_or("interface identifier must include an @version suffix")?;
        if identity.is_empty() {
            return Err("interface identifier identity must not be empty");
        }
        if identity.contains('@') {
            return Err("interface identifier must contain exactly one @version suffix");
        }
        let version = version
            .parse::<u64>()
            .map_err(|_| "interface identifier version must be a positive integer")?;
        if version == 0 {
            return Err("interface identifier version must be a positive integer");
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Display for InterfaceId {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl Serialize for InterfaceId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for InterfaceId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::parse(value).map_err(D::Error::custom)
    }
}

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

    #[test]
    fn runtime_interface_ids_require_an_explicit_positive_version() {
        let id = InterfaceId::parse("phenix.models.inference@1").unwrap();
        assert_eq!(id.as_str(), "phenix.models.inference@1");
        assert!(InterfaceId::parse("phenix.models.inference").is_err());
        assert!(InterfaceId::parse("phenix.models.inference@0").is_err());
        assert!(InterfaceId::parse("phenix.models.inference@latest").is_err());
        assert!(InterfaceId::parse("phenix.models@inference@1").is_err());
        assert!(InterfaceId::parse("@1").is_err());
    }

    #[test]
    fn identifiers_round_trip_as_canonical_strings() {
        let id = ComponentId::parse("phenix.agent@1").unwrap();
        let encoded = serde_json::to_string(&id).unwrap();
        assert_eq!(encoded, "\"phenix.agent@1\"");
        assert_eq!(serde_json::from_str::<ComponentId>(&encoded).unwrap(), id);
    }

    #[test]
    fn deserialization_preserves_identifier_validation() {
        assert!(serde_json::from_str::<PluginId>("\"has space\"").is_err());
        assert!(serde_json::from_str::<PluginId>("\"\"").is_err());
        assert!(serde_json::from_str::<InterfaceId>("\"unversioned\"").is_err());
        assert!(serde_json::from_str::<InterfaceId>("\"phenix.models@inference@1\"").is_err());
        assert!(serde_json::from_str::<InterfaceId>("\"phenix.models.inference@0\"").is_err());
    }
}
