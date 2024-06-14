mod v0;

use crate::config::QuorumLikeConfig;
use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform_types::core_quorum_set::v0::for_saving::CoreQuorumSetForSavingV0;
pub use crate::platform_types::core_quorum_set::v0::quorum_set::{
    CoreQuorumSetV0, CoreQuorumSetV0Methods, QuorumConfig, QuorumsWithConfig,
    SelectedQuorumSetIterator, SIGN_OFFSET,
};
pub use crate::platform_types::core_quorum_set::v0::quorums::{
    Quorum, Quorums, SigningQuorum, ThresholdBlsPublicKey, VerificationQuorum,
};
use bincode::{Decode, Encode};
use derive_more::From;
use dpp::version::PlatformVersion;

/// Quorums with keys for signature verification
#[derive(Debug, Clone, From)]
pub enum CoreQuorumSet {
    /// Version 0 of the signature verification quorums
    V0(CoreQuorumSetV0),
}

impl CoreQuorumSet {
    /// Create a default SignatureVerificationQuorums
    pub fn new(
        config: &impl QuorumLikeConfig,
        platform_version: &PlatformVersion,
    ) -> Result<Self, Error> {
        match platform_version.drive_abci.structs.core_quorum_set {
            0 => Ok(CoreQuorumSetV0::new(config).into()),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "CoreQuorumSet.new".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
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

    fn select_quorums_for_heights(
        &self,
        signing_height: u32,
        verification_height: u32,
    ) -> SelectedQuorumSetIterator {
        match self {
            Self::V0(v0) => v0.select_quorums_for_heights(signing_height, verification_height),
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
