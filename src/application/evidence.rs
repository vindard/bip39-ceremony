use crate::domain::{bip39::EntropyTarget, ceremony::CeremonyState, protocol::ConversionProtocol};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Capability {
    Absent,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BuildCapabilities {
    version: &'static str,
    network_io: Capability,
    ceremony_persistence: Capability,
}

impl BuildCapabilities {
    #[must_use]
    pub const fn current() -> Self {
        Self {
            version: env!("CARGO_PKG_VERSION"),
            network_io: Capability::Absent,
            ceremony_persistence: Capability::Absent,
        }
    }

    #[must_use]
    pub const fn version(self) -> &'static str {
        self.version
    }
    #[must_use]
    pub const fn network_io(self) -> Capability {
        self.network_io
    }
    #[must_use]
    pub const fn ceremony_persistence(self) -> Capability {
        self.ceremony_persistence
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ReproductionReceipt {
    protocol: ConversionProtocol,
    target: EntropyTarget,
    input_count: usize,
}

impl ReproductionReceipt {
    #[must_use]
    pub fn from_state(state: &CeremonyState) -> Option<Self> {
        Some(Self {
            protocol: state.protocol()?,
            target: state.target()?,
            input_count: state.capture_count(),
        })
    }

    #[must_use]
    pub const fn protocol(self) -> ConversionProtocol {
        self.protocol
    }
    #[must_use]
    pub const fn target(self) -> EntropyTarget {
        self.target
    }
    #[must_use]
    pub const fn input_count(self) -> usize {
        self.input_count
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{
        ceremony::{Ceremony, Command},
        protocol::ConversionProtocol,
    };

    #[test]
    fn receipt_contains_only_non_secret_reproduction_metadata() {
        let mut ceremony = Ceremony::new();
        ceremony
            .handle(Command::SelectTarget(EntropyTarget::Words12))
            .unwrap();
        ceremony
            .handle(Command::SelectProtocol(ConversionProtocol::ExactV1))
            .unwrap();
        let receipt = ReproductionReceipt::from_state(&ceremony.state()).unwrap();
        assert_eq!(receipt.protocol(), ConversionProtocol::ExactV1);
        assert_eq!(receipt.target(), EntropyTarget::Words12);
        assert_eq!(receipt.input_count(), 0);
        assert!(!format!("{receipt:?}").contains("faces"));
    }
}
