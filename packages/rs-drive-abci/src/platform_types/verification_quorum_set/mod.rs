mod v0;

use crate::config::QuorumLikeConfig;
use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform_types::verification_quorum_set::v0::for_saving::VerificationQuorumSetForSavingV0;
pub use crate::platform_types::verification_quorum_set::v0::quorum_set::{
    QuorumConfig, QuorumsWithConfig, SelectedQuorumSetIterator, VerificationQuorumSetV0,
    VerificationQuorumSetV0Methods, SIGN_OFFSET,
};
pub use crate::platform_types::verification_quorum_set::v0::quorums::{
    Quorum, Quorums, SigningQuorum, ThresholdBlsPublicKey, VerificationQuorum,
};
use bincode::{Decode, Encode};
use derive_more::From;
use dpp::version::PlatformVersion;

/// Quorums with keys for signature verification
#[derive(Debug, Clone, From)]
pub enum VerificationQuorumSet {
    /// Version 0 of the signature verification quorums
    V0(VerificationQuorumSetV0),
}

impl VerificationQuorumSet {
    /// Create a default SignatureVerificationQuorums
    pub fn new(
        config: &impl QuorumLikeConfig,
        platform_version: &PlatformVersion,
    ) -> Result<Self, Error> {
        match platform_version.drive_abci.structs.verification_quorum_set {
            0 => Ok(VerificationQuorumSetV0::new(config).into()),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "VerificationQuorumSet.new".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}

impl VerificationQuorumSetV0Methods for VerificationQuorumSet {
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

    fn has_previous_past_quorums(&self) -> bool {
        match self {
            Self::V0(v0) => v0.has_previous_past_quorums(),
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

    fn set_previous_past_quorums(
        &mut self,
        previous_quorums: Quorums<VerificationQuorum>,
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
    ) -> SelectedQuorumSetIterator {
        match self {
            Self::V0(v0) => v0.select_quorums(signing_height, verification_height),
        }
    }
}

/// Core Quorum Set structure for saving to the database
#[derive(Debug, Clone, Encode, Decode)]
pub enum VerificationQuorumSetForSaving {
    /// Version 0 of the signature verification quorums
    V0(VerificationQuorumSetForSavingV0),
}

impl From<VerificationQuorumSet> for VerificationQuorumSetForSaving {
    fn from(value: VerificationQuorumSet) -> Self {
        match value {
            VerificationQuorumSet::V0(v0) => VerificationQuorumSetForSaving::V0(v0.into()),
        }
    }
}

impl From<VerificationQuorumSetForSaving> for VerificationQuorumSet {
    fn from(value: VerificationQuorumSetForSaving) -> Self {
        match value {
            VerificationQuorumSetForSaving::V0(v0) => VerificationQuorumSet::V0(v0.into()),
        }
    }
}
