use crate::error::PlatformVersionError;
use crate::version::fee::data_contract_registration::v1::FEE_DATA_CONTRACT_REGISTRATION_VERSION1;
use crate::version::fee::data_contract_registration::FeeDataContractRegistrationVersion;
use crate::version::fee::data_contract_validation::FeeDataContractValidationVersion;
use crate::version::fee::hashing::v1::FEE_HASHING_VERSION1;
use crate::version::fee::hashing::{FeeHashingVersion, FeeHashingVersionBeforeVersion11};
use crate::version::fee::processing::{
    FeeProcessingVersion, FeeProcessingVersionFieldsBeforeVersion1Point4,
};
use crate::version::fee::signature::FeeSignatureVersion;
use crate::version::fee::state_transition_min_fees::{
    StateTransitionMinFees, StateTransitionMinFeesBeforeProtocolVersion11,
};
use crate::version::fee::storage::FeeStorageVersion;
use crate::version::fee::v1::FEE_VERSION1;
use crate::version::fee::vote_resolution_fund_fees::VoteResolutionFundFees;
use bincode::{Decode, Encode};

pub mod data_contract_registration;
mod data_contract_validation;
mod hashing;
mod processing;
pub mod signature;
pub mod state_transition_min_fees;
pub mod storage;
pub mod v1;
pub mod v2;
pub mod vote_resolution_fund_fees;

pub type FeeVersionNumber = u32;

/// The fee schedules [`FeeVersion::get`] can resolve, indexed by `fee_version_number - 1`.
///
/// # This list is INCOMPLETE, and that is a known defect
///
/// `FEE_VERSION2` — what protocol versions 9 and later actually run with — is missing, and
/// declares `fee_version_number: 1`, colliding with `FEE_VERSION1`. Since the fee version
/// NUMBER is the only thing persisted (`PlatformStateForSavingV1` and
/// `ReducedPlatformStateV0` both store `epoch index -> number`), every node that restarts
/// or state-syncs rehydrates previous epochs' fees as `FEE_VERSION1`. See the doc comment
/// on [`v2::FEE_VERSION2`] for why that is currently latent and what fixing it requires.
pub const FEE_VERSIONS: &[FeeVersion] = &[FEE_VERSION1];

#[derive(Clone, Debug, Encode, Decode, Default, PartialEq, Eq)]
pub struct FeeVersion {
    pub fee_version_number: FeeVersionNumber,
    // Permille means devise by 1000
    pub uses_version_fee_multiplier_permille: Option<u64>,
    pub storage: FeeStorageVersion,
    pub signature: FeeSignatureVersion,
    pub hashing: FeeHashingVersion,
    pub processing: FeeProcessingVersion,
    pub data_contract_validation: FeeDataContractValidationVersion,
    pub data_contract_registration: FeeDataContractRegistrationVersion,
    pub state_transition_min_fees: StateTransitionMinFees,
    pub vote_resolution_fund_fees: VoteResolutionFundFees,
}

impl FeeVersion {
    pub fn as_static(&self) -> &'static FeeVersion {
        FeeVersion::get(self.fee_version_number).expect("expected fee version to exist")
    }
    pub fn get<'a>(version: FeeVersionNumber) -> Result<&'a Self, PlatformVersionError> {
        if version > 0 {
            FEE_VERSIONS.get(version as usize - 1).ok_or_else(|| {
                PlatformVersionError::UnknownVersionError(format!("no fee version {version}"))
            })
        } else {
            Err(PlatformVersionError::UnknownVersionError(format!(
                "no fee version {version}"
            )))
        }
    }

    pub fn get_optional<'a>(version: FeeVersionNumber) -> Option<&'a Self> {
        if version > 0 {
            FEE_VERSIONS.get(version as usize - 1)
        } else {
            None
        }
    }

    pub fn first<'a>() -> &'a Self {
        FEE_VERSIONS
            .first()
            .expect("expected to have a fee version")
    }

    pub fn latest<'a>() -> &'a Self {
        FEE_VERSIONS.last().expect("expected to have a fee version")
    }
}

// This is type only meant for deserialization because of an issue
// The issue was that the platform state was stored with FeeVersions in it before version 1.4
// When we would add new fields we would be unable to deserialize
// This FeeProcessingVersionFieldsBeforeVersion4 is how things were before version 1.4 was released
#[derive(Clone, Debug, Encode, Decode, Default, PartialEq, Eq)]
pub struct FeeVersionFieldsBeforeVersion4 {
    // Permille means devise by 1000
    pub uses_version_fee_multiplier_permille: Option<u64>,
    pub storage: FeeStorageVersion,
    pub signature: FeeSignatureVersion,
    pub hashing: FeeHashingVersionBeforeVersion11,
    pub processing: FeeProcessingVersionFieldsBeforeVersion1Point4,
    pub data_contract: FeeDataContractValidationVersion,
    pub state_transition_min_fees: StateTransitionMinFeesBeforeProtocolVersion11,
    pub vote_resolution_fund_fees: VoteResolutionFundFees,
}

impl From<FeeVersionFieldsBeforeVersion4> for FeeVersion {
    fn from(value: FeeVersionFieldsBeforeVersion4) -> Self {
        FeeVersion {
            fee_version_number: 1,
            uses_version_fee_multiplier_permille: value.uses_version_fee_multiplier_permille,
            storage: value.storage,
            signature: value.signature,
            hashing: FEE_HASHING_VERSION1,
            processing: FeeProcessingVersion::from(value.processing),
            data_contract_validation: value.data_contract,
            data_contract_registration: FEE_DATA_CONTRACT_REGISTRATION_VERSION1,
            state_transition_min_fees: StateTransitionMinFees::from(
                value.state_transition_min_fees,
            ),
            vote_resolution_fund_fees: value.vote_resolution_fund_fees,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::version::fee::v2::FEE_VERSION2;

    /// Every `FeeVersion` constant must carry a distinct `fee_version_number`, and
    /// `FEE_VERSIONS` must contain all of them, because the number is the ONLY thing
    /// persisted: `PlatformStateForSavingV1` and `ReducedPlatformStateV0` both store
    /// `(epoch index -> fee version number)` and rehydrate through `FeeVersion::get`. A
    /// number that does not resolve back to the constant it came from silently substitutes
    /// a different fee schedule on any node that restarts or state-syncs.
    ///
    /// This test FAILS today, which is why it is ignored: `FEE_VERSION2` declares
    /// `fee_version_number: 1`, the same as `FEE_VERSION1`, and is absent from
    /// `FEE_VERSIONS`, so `FeeVersion::get(1)` returns `FEE_VERSION1` even for the epochs
    /// that ran on `FEE_VERSION2`. See the doc comment on `FEE_VERSION2`.
    ///
    /// Un-ignore it as part of giving `FEE_VERSION2` its own number and adding it to
    /// `FEE_VERSIONS`. That is protocol-visible and needs a migration, which is why the
    /// defect is pinned here rather than fixed in place.
    #[test]
    #[ignore = "known defect: FEE_VERSION2 reuses fee_version_number 1 and is absent from \
                FEE_VERSIONS; fixing it is protocol-visible - see the FEE_VERSION2 docs"]
    fn fee_version_numbers_are_unique_and_resolvable() {
        let all_fee_versions = [&FEE_VERSION1, &FEE_VERSION2];

        for fee_version in all_fee_versions {
            let resolved = FeeVersion::get(fee_version.fee_version_number).unwrap_or_else(|_| {
                panic!(
                    "fee version number {} does not resolve through FEE_VERSIONS",
                    fee_version.fee_version_number
                )
            });
            assert_eq!(
                resolved, fee_version,
                "FeeVersion::get({}) returned a DIFFERENT fee schedule than the constant \
                 declaring that number. Every number-only round trip - a node restarting, a \
                 node state-syncing - would substitute this wrong schedule.",
                fee_version.fee_version_number
            );
        }

        let mut numbers: Vec<FeeVersionNumber> = all_fee_versions
            .iter()
            .map(|fee_version| fee_version.fee_version_number)
            .collect();
        numbers.sort_unstable();
        let mut deduped = numbers.clone();
        deduped.dedup();
        assert_eq!(
            deduped.len(),
            numbers.len(),
            "two FeeVersion constants share a fee_version_number: {:?}",
            numbers
        );
    }
}
