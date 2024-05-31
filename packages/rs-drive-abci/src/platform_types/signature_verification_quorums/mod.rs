mod v0;

use crate::platform_types::signature_verification_quorums::v0::{
    PreviousQuorums, SelectedVerificationQuorumSets,
};
use bincode::{Decode, Encode};
use derive_more::From;
use dpp::version::PlatformVersion;
pub use v0::{QuorumKeys, SignatureVerificationQuorumsV0, SignatureVerificationQuorumsV0Methods};

/// Quorums with keys for signature verification
#[derive(Debug, Clone, Encode, Decode, From)]
pub enum SignatureVerificationQuorums {
    /// Version 0 of the signature verification quorums
    V0(SignatureVerificationQuorumsV0),
}

impl SignatureVerificationQuorums {
    /// Create a default SignatureVerificationQuorums
    pub fn default_for_platform_version(platform_version: &PlatformVersion) -> Self {
        // TODO: default for platform version

        SignatureVerificationQuorumsV0::default().into()
    }
}

impl SignatureVerificationQuorumsV0Methods for SignatureVerificationQuorums {
    fn set_last_quorums(&mut self, quorums: QuorumKeys) {
        match self {
            Self::V0(v0) => v0.set_last_quorums(quorums),
        }
    }

    fn last_quorums(&self) -> &QuorumKeys {
        match self {
            Self::V0(v0) => v0.last_quorums(),
        }
    }

    fn last_quorums_mut(&mut self) -> &mut QuorumKeys {
        match self {
            Self::V0(v0) => v0.last_quorums_mut(),
        }
    }

    fn previous_quorums(&self) -> Option<&PreviousQuorums> {
        match self {
            Self::V0(v0) => v0.previous_quorums(),
        }
    }

    fn rotate_quorums(
        &mut self,
        quorums: QuorumKeys,
        last_active_core_height: u32,
        updated_at_core_height: u32,
    ) {
        match self {
            Self::V0(v0) => {
                v0.rotate_quorums(quorums, last_active_core_height, updated_at_core_height)
            }
        }
    }

    fn update_previous_quorums(
        &mut self,
        previous_quorums: QuorumKeys,
        last_active_core_height: u32,
        updated_at_core_height: u32,
    ) {
        match self {
            Self::V0(v0) => v0.update_previous_quorums(
                previous_quorums,
                last_active_core_height,
                updated_at_core_height,
            ),
        }
    }

    fn select_quorums(
        &self,
        signing_height: u32,
        verification_height: u32,
    ) -> SelectedVerificationQuorumSets {
        match self {
            Self::V0(v0) => v0.select_quorums(signing_height, verification_height),
        }
    }
}
