mod v0;

use crate::config::QuorumLikeConfig;
use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform_types::signature_verification_quorum_set::v0::for_saving_v2::SignatureVerificationQuorumSetForSavingV2;
pub use crate::platform_types::signature_verification_quorum_set::v0::quorum_set::{
    QuorumConfig, QuorumsWithConfig, SelectedQuorumSetIterator, SignatureVerificationQuorumSetV0,
    SignatureVerificationQuorumSetV0Methods, SIGN_OFFSET,
};
pub use crate::platform_types::signature_verification_quorum_set::v0::quorums::{
    Quorum, Quorums, SigningQuorum, ThresholdBlsPublicKey, VerificationQuorum,
};
use crate::platform_types::unsupported_legacy_bls_storage::UnsupportedLegacyBlsStorage;
use bincode::{Decode, Encode};
use derive_more::From;
use dpp::version::PlatformVersion;

/// Quorums with keys for signature verification
#[derive(Debug, Clone, From)]
pub enum SignatureVerificationQuorumSet {
    /// Version 0 of the signature verification quorums
    V0(SignatureVerificationQuorumSetV0),
}

impl SignatureVerificationQuorumSet {
    /// Create a default SignatureVerificationQuorums
    pub fn new(
        config: &impl QuorumLikeConfig,
        platform_version: &PlatformVersion,
    ) -> Result<Self, Error> {
        match platform_version
            .drive_abci
            .structs
            .signature_verification_quorum_set
        {
            0 => Ok(SignatureVerificationQuorumSetV0::new(config).into()),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "SignatureVerificationQuorumSet.new".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}

impl SignatureVerificationQuorumSetV0Methods for SignatureVerificationQuorumSet {
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
    ) -> SelectedQuorumSetIterator<'_> {
        match self {
            Self::V0(v0) => v0.select_quorums(signing_height, verification_height),
        }
    }
}

/// Core Quorum Set structure for saving to the database
#[derive(Debug, Clone, Encode, Decode)]
pub enum SignatureVerificationQuorumSetForSaving {
    /// Unsupported legacy format; its tag remains reserved.
    V0(UnsupportedLegacyBlsStorage),
    /// Unsupported legacy format; its tag remains reserved.
    V1(UnsupportedLegacyBlsStorage),
    /// Version 2 of the signature verification quorums
    V2(SignatureVerificationQuorumSetForSavingV2),
}

impl From<SignatureVerificationQuorumSet> for SignatureVerificationQuorumSetForSaving {
    fn from(value: SignatureVerificationQuorumSet) -> Self {
        match value {
            SignatureVerificationQuorumSet::V0(v0) => {
                SignatureVerificationQuorumSetForSaving::V2(v0.into())
            }
        }
    }
}

impl From<SignatureVerificationQuorumSetForSaving> for SignatureVerificationQuorumSet {
    fn from(value: SignatureVerificationQuorumSetForSaving) -> Self {
        match value {
            SignatureVerificationQuorumSetForSaving::V0(unsupported)
            | SignatureVerificationQuorumSetForSaving::V1(unsupported) => match unsupported {},
            SignatureVerificationQuorumSetForSaving::V2(v2) => {
                SignatureVerificationQuorumSet::V0(v2.into())
            }
        }
    }
}

#[cfg(test)]
mod legacy_storage_tests {
    use super::SignatureVerificationQuorumSetForSaving;
    use bincode::{config, error::DecodeError};

    #[test]
    fn should_reject_legacy_bls_quorum_formats_before_reading_their_payload() {
        for version in [0u32, 1] {
            let bytes = bincode::encode_to_vec(version, config::standard().with_big_endian())
                .expect("encode format tag");
            let error = bincode::decode_from_slice::<SignatureVerificationQuorumSetForSaving, _>(
                &bytes,
                config::standard().with_big_endian(),
            )
            .expect_err("legacy quorum format must be rejected");
            let borrowed_error = bincode::borrow_decode_from_slice::<
                SignatureVerificationQuorumSetForSaving,
                _,
            >(&bytes, config::standard().with_big_endian())
            .expect_err("borrowed decoding must reject legacy quorums too");
            assert_eq!(error.to_string(), borrowed_error.to_string());
            assert!(matches!(error, DecodeError::Other(_)));
            assert!(error.to_string().contains("v4.0.0"));
            assert!(error.to_string().contains("commit at least one block"));
        }
    }
}
