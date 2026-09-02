#![forbid(unsafe_code)]

macro_rules! domain_id_type {
    ($name:ident) => {
        #[derive(
            Clone,
            Debug,
            Eq,
            Ord,
            PartialEq,
            PartialOrd,
            Hash,
            serde::Serialize,
            serde::Deserialize,
        )]
        #[serde(try_from = "String")]
        pub struct $name(String);

        impl $name {
            pub fn parse(value: impl Into<String>) -> Result<Self, $crate::InvalidId> {
                value.into().try_into()
            }

            #[must_use]
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl AsRef<str> for $name {
            fn as_ref(&self) -> &str {
                self.as_str()
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str(self.as_str())
            }
        }

        impl std::str::FromStr for $name {
            type Err = $crate::InvalidId;

            fn from_str(value: &str) -> Result<Self, Self::Err> {
                value.to_owned().try_into()
            }
        }

        impl TryFrom<String> for $name {
            type Error = $crate::InvalidId;

            fn try_from(value: String) -> Result<Self, Self::Error> {
                if value.trim().is_empty() {
                    return Err($crate::InvalidId);
                }
                Ok(Self(value))
            }
        }
    };
}

mod attempts;
mod debug;
mod failures;
mod workspace;

pub use attempts::*;
pub use debug::*;
pub use failures::*;
pub use phenix_core::{
    CallableId, Key, ModelId, PhenixSchema, PhenixValue, RoutingProfileId, SkillId,
};
pub use workspace::*;

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::{self, Display, Formatter};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InvalidId;

impl Display for InvalidId {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str("identifier must not be empty")
    }
}

impl std::error::Error for InvalidId {}

// ... remainder omitted?