mod v0;

use crate::platform_types::signature_verification_quorums::v0::{
    PreviousPastQuorums, SelectedVerificationQuorumSets,
};
use bincode::{Decode, Encode};
use derive_more::From;
use dpp::version::PlatformVersion;
pub use v0::{
    QuorumKeysByQuorumHash, SignatureVerificationQuorumsV0, SignatureVerificationQuorumsV0Methods,
};

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
    fn set_current_quorums(&mut self, quorums: QuorumKeysByQuorumHash) {
        match self {
            Self::V0(v0) => v0.set_current_quorums(quorums),
        }
    }

    fn current_quorums(&self) -> &QuorumKeysByQuorumHash {
        match self {
            Self::V0(v0) => v0.current_quorums(),
        }
    }

    fn current_quorums_mut(&mut self) -> &mut QuorumKeysByQuorumHash {
        match self {
            Self::V0(v0) => v0.current_quorums_mut(),
        }
    }

    fn previous_past_quorums(&self) -> Option<&PreviousPastQuorums> {
        match self {
            Self::V0(v0) => v0.previous_past_quorums(),
        }
    }

    fn rotate_quorums(
        &mut self,
        quorums: QuorumKeysByQuorumHash,
        last_active_core_height: u32,
        updated_at_core_height: u32,
    ) {
        match self {
            Self::V0(v0) => {
                v0.rotate_quorums(quorums, last_active_core_height, updated_at_core_height)
            }
        }
    }

    fn set_previous_past_quorums(
        &mut self,
        previous_quorums: QuorumKeysByQuorumHash,
        last_active_core_height: u32,
        updated_at_core_height: u32,
    ) {
        match self {
            Self::V0(v0) => v0.set_previous_past_quorums(
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
