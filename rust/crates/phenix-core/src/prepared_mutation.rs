use crate::{
    Authority, Exact, GraphGenerationId, NamespaceTransaction, PhenixValue, PluginId, Project,
    ResourceNamespace, StoreBindingId, TransactionOp, Type, TypeKind, ValueCodec, ValueError,
};
use serde::{Deserialize, Deserializer, Serialize};
use std::{
    collections::BTreeMap,
    fmt::{self, Display, Formatter},
    sync::Mutex,
};

const HANDLE_PREFIX: &str = "pm:";
const HANDLE_HEX_LENGTH: usize = 64;

/// Opaque capability for one owner-prepared durable mutation.
///
/// The identifier is intentionally insufficient on its own. Core resolves it only in the
/// transaction scope that created it and consumes it on the first commit attempt.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct PreparedMutationHandle(String);

impl PreparedMutationHandle {
    fn generate() -> Result<Self, String> {
        let mut random = [0_u8; 32];
        getrandom::fill(&mut random)
            .map_err(|error| format!("cannot generate prepared mutation handle: {error}"))?;
        let mut value = String::with_capacity(HANDLE_PREFIX.len() + HANDLE_HEX_LENGTH);
        value.push_str(HANDLE_PREFIX);
        for byte in random {
            use std::fmt::Write as _;
            write!(&mut value, "{byte:02x}").expect("writing to String cannot fail");
        }
        Ok(Self(value))
    }

    fn parse(value: String) -> Result<Self, &'static str> {
        let Some(digest) = value.strip_prefix(HANDLE_PREFIX) else {
            return Err("prepared mutation handle has an invalid prefix");
        };
        if digest.len() != HANDLE_HEX_LENGTH
            || !digest
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            return Err("prepared mutation handle must contain 64 lowercase hexadecimal digits");
        }
        Ok(Self(value))
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

impl Display for PreparedMutationHandle {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for PreparedMutationHandle {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Self::parse(String::deserialize(deserializer)?).map_err(serde::de::Error::custom)
    }
}

impl ValueCodec for PreparedMutationHandle {
    fn phenix_type() -> Type {
        Type::String
    }

    fn to_value(&self) -> PhenixValue {
        PhenixValue::String(self.0.clone())
    }

    fn from_value(value: &PhenixValue) -> Result<Self, ValueError> {
        match value {
            PhenixValue::String(value) => Self::parse(value.clone())
                .map_err(|error| ValueError::InvalidValue(error.into())),
            _ => Err(ValueError::TypeMismatch {
                expected: TypeKind::String,
                actual: value.kind(),
            }),
        }
    }
}

impl From<&PreparedMutationHandle> for PhenixValue {
    fn from(value: &PreparedMutationHandle) -> Self {
        <PreparedMutationHandle as ValueCodec>::to_value(value)
    }
}

impl<'value> TryFrom<Exact<&'value PhenixValue>> for PreparedMutationHandle {
    type Error = ValueError;

    fn try_from(value: Exact<&'value PhenixValue>) -> Result<Self, Self::Error> {
        <Self as ValueCodec>::from_value(value.0)
    }
}

impl<'value> TryFrom<Project<&'value PhenixValue>> for PreparedMutationHandle {
    type Error = ValueError;

    fn try_from(value: Project<&'value PhenixValue>) -> Result<Self, Self::Error> {
        <Self as ValueCodec>::project_from_value(value.0)
    }
}

#[derive(Clone, Debug)]
pub(crate) struct PreparedMutation {
    pub(crate) transaction: NamespaceTransaction,
    pub(crate) authority: Authority,
}

pub(crate) struct PreparedMutationScope {
    generation: Option<GraphGenerationId>,
    store_binding: Option<StoreBindingId>,
    prepared: Mutex<BTreeMap<PreparedMutationHandle, PreparedMutation>>,
}

impl PreparedMutationScope {
    pub(crate) fn new(
        generation: Option<&GraphGenerationId>,
        store_binding: Option<&StoreBindingId>,
    ) -> Self {
        Self {
            generation: generation.cloned(),
            store_binding: store_binding.cloned(),
            prepared: Mutex::new(BTreeMap::new()),
        }
    }

    pub(crate) fn generation(&self) -> Option<&GraphGenerationId> {
        self.generation.as_ref()
    }

    pub(crate) fn store_binding(&self) -> Option<&StoreBindingId> {
        self.store_binding.as_ref()
    }

    pub(crate) fn prepare(
        &self,
        owner: &PluginId,
        namespace: &ResourceNamespace,
        operations: &[TransactionOp],
        authority: &Authority,
    ) -> Result<PreparedMutationHandle, String> {
        let mut prepared = self
            .prepared
            .lock()
            .expect("prepared mutation registry mutex poisoned");
        loop {
            let handle = PreparedMutationHandle::generate()?;
            if prepared.contains_key(&handle) {
                continue;
            }
            prepared.insert(
                handle.clone(),
                PreparedMutation {
                    transaction: NamespaceTransaction {
                        owner: owner.clone(),
                        namespace: namespace.clone(),
                        operations: operations.to_vec(),
                    },
                    authority: authority.clone(),
                },
            );
            return Ok(handle);
        }
    }

    /// Consume every supplied handle before validation or backend commit.
    ///
    /// This deliberately makes every commit attempt one-shot. If a participant is missing,
    /// invalid, cancelled, or fails an assertion, callers must ask each owner to prepare again.
    pub(crate) fn consume(
        &self,
        handles: &[PreparedMutationHandle],
    ) -> Result<Vec<PreparedMutation>, PreparedMutationHandle> {
        let mut prepared = self
            .prepared
            .lock()
            .expect("prepared mutation registry mutex poisoned");
        let mut participants = Vec::with_capacity(handles.len());
        let mut missing = None;
        for handle in handles {
            match prepared.remove(handle) {
                Some(participant) => participants.push(participant),
                None if missing.is_none() => missing = Some(handle.clone()),
                None => {}
            }
        }
        match missing {
            Some(handle) => Err(handle),
            None => Ok(participants),
        }
    }

    pub(crate) fn clear(&self) {
        self.prepared
            .lock()
            .expect("prepared mutation registry mutex poisoned")
            .clear();
    }

    #[cfg(test)]
    pub(crate) fn outstanding(&self) -> usize {
        self.prepared
            .lock()
            .expect("prepared mutation registry mutex poisoned")
            .len()
    }
}
