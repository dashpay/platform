mod v0;

use crate::config::QuorumLikeConfig;
use crate::platform_types::core_quorum_set::v0::for_saving::CoreQuorumSetForSavingV0;
pub use crate::platform_types::core_quorum_set::v0::quorum_set::{
    CoreQuorumSetV0, CoreQuorumSetV0Methods, QuorumConfig, QuorumsVerificationDataIterator,
};
pub use crate::platform_types::core_quorum_set::v0::quorums::{
    Quorum, Quorums, ReversedQuorumHashBytes, SigningQuorum, ThresholdBlsPublicKey,
    VerificationQuorum,
};
use bincode::{Decode, Encode};
use derive_more::From;
use dpp::dashcore::QuorumSigningRequestId;
use dpp::version::PlatformVersion;

/// Quorums with keys for signature verification
#[derive(Debug, Clone, From)]
pub enum CoreQuorumSet {
    /// Version 0 of the signature verification quorums
    V0(CoreQuorumSetV0),
}

impl CoreQuorumSet {
    /// Create a default SignatureVerificationQuorums
    pub fn new(config: &impl QuorumLikeConfig, platform_version: &PlatformVersion) -> Self {
        // TODO: default for platform version

        CoreQuorumSetV0::new(config).into()
    }
}

impl CoreQuorumSetV0Methods for CoreQuorumSet {
    fn config(&self) -> &QuorumConfig {
        match self {
            Self::V0(v0) => v0.config(),
        }
    }

    fn set_current_quorums(&mut self, quorums: Quorums<VerificationQuorum>) {
        match self {
            Self::V0(v0) => v0.set_current_quorums(quorums),
        }
    }

    fn current_quorums(&self) -> &Quorums<VerificationQuorum> {
        match self {
            Self::V0(v0) => v0.current_quorums(),
        }
    }

    fn current_quorums_mut(&mut self) -> &mut Quorums<VerificationQuorum> {
        match self {
            Self::V0(v0) => v0.current_quorums_mut(),
        }
    }

    fn has_previous_quorums(&self) -> bool {
        match self {
            Self::V0(v0) => v0.has_previous_quorums(),
        }
    }

    fn replace_quorums(
        &mut self,
        quorums: Quorums<VerificationQuorum>,
        last_active_core_height: u32,
        updated_at_core_height: u32,
    ) {
        match self {
            Self::V0(v0) => {
                v0.replace_quorums(quorums, last_active_core_height, updated_at_core_height)
            }
        }
    }

    fn update_previous_quorums(
        &mut self,
        previous_quorums: Quorums<VerificationQuorum>,
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
        request_id: QuorumSigningRequestId,
    ) -> QuorumsVerificationDataIterator {
        match self {
            Self::V0(v0) => v0.select_quorums(signing_height, verification_height, request_id),
        }
    }
}

/// Core Quorum Set structure for saving to the database
#[derive(Debug, Clone, Encode, Decode)]
pub enum CoreQuorumSetForSaving {
    /// Version 0 of the signature verification quorums
    V0(CoreQuorumSetForSavingV0),
}

impl From<CoreQuorumSet> for CoreQuorumSetForSaving {
    fn from(value: CoreQuorumSet) -> Self {
        match value {
            CoreQuorumSet::V0(v0) => CoreQuorumSetForSaving::V0(v0.into()),
        }
    }
}

impl From<CoreQuorumSetForSaving> for CoreQuorumSet {
    fn from(value: CoreQuorumSetForSaving) -> Self {
        match value {
            CoreQuorumSetForSaving::V0(v0) => CoreQuorumSet::V0(v0.into()),
        }
    }
}
