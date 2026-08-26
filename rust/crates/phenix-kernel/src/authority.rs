use crate::CapabilityId;
use std::collections::BTreeSet;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Authority {
    capabilities: BTreeSet<CapabilityId>,
}

impl Authority {
    pub fn new(capabilities: impl IntoIterator<Item = CapabilityId>) -> Self {
        Self {
            capabilities: capabilities.into_iter().collect(),
        }
    }

    pub fn permits(&self, capability: &CapabilityId) -> bool {
        self.capabilities.contains(capability)
    }

    pub fn permits_all(&self, required: &Self) -> bool {
        required
            .capabilities
            .iter()
            .all(|capability| self.permits(capability))
    }

    pub fn attenuate(&self, requested: &Self) -> Self {
        Self {
            capabilities: self
                .capabilities
                .intersection(&requested.capabilities)
                .cloned()
                .collect(),
        }
    }

    pub fn capabilities(&self) -> impl Iterator<Item = &CapabilityId> {
        self.capabilities.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cap(value: &str) -> CapabilityId {
        CapabilityId::parse(value).unwrap()
    }

    #[test]
    fn attenuation_cannot_regain_authority() {
        let parent = Authority::new([cap("fs.read"), cap("network.read")]);
        let requested = Authority::new([cap("fs.read"), cap("fs.write")]);
        let child = parent.attenuate(&requested);

        assert!(child.permits(&cap("fs.read")));
        assert!(!child.permits(&cap("fs.write")));
        assert!(!child.permits(&cap("network.read")));
    }
}
