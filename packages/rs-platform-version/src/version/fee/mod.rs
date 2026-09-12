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
#[cfg(feature = "mock-versions")]
use crate::version::mocks::fee_doubled_storage_test::TEST_FEE_VERSIONS;
#[cfg(feature = "mock-versions")]
use crate::version::mocks::TEST_PROTOCOL_VERSION_SHIFT_BYTES;
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
            #[cfg(feature = "mock-versions")]
            {
                // Test fee generations share the mock protocol versions' shifted
                // number range, so a number with the test bit set is resolved by
                // number in the test registry and never reaches the production one.
                if version >> TEST_PROTOCOL_VERSION_SHIFT_BYTES > 0 {
                    return TEST_FEE_VERSIONS
                        .iter()
                        .find(|fee_version| fee_version.fee_version_number == version)
                        .ok_or_else(|| {
                            PlatformVersionError::UnknownVersionError(format!(
                                "no test fee version {version}"
                            ))
                        });
                }
            }
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
            #[cfg(feature = "mock-versions")]
            {
                if version >> TEST_PROTOCOL_VERSION_SHIFT_BYTES > 0 {
                    return TEST_FEE_VERSIONS
                        .iter()
                        .find(|fee_version| fee_version.fee_version_number == version);
                }
            }
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

#[cfg(all(test, feature = "mock-versions"))]
mod mock_fee_generation_tests {
    use super::{FeeStorageVersion, FeeVersion, FEE_VERSIONS};
    use crate::version::mocks::fee_doubled_storage_test::{
        TEST_FEE_VERSION_DOUBLED_STORAGE, TEST_FEE_VERSION_NUMBER_DOUBLED_STORAGE,
    };
    use crate::version::mocks::TEST_PROTOCOL_VERSION_SHIFT_BYTES;
    use crate::version::PlatformVersion;

    #[test]
    fn should_resolve_the_test_fee_generation_only_through_the_shifted_number_range() {
        let resolved = FeeVersion::get(TEST_FEE_VERSION_NUMBER_DOUBLED_STORAGE)
            .expect("the test fee generation resolves by its number");
        assert_eq!(resolved, &TEST_FEE_VERSION_DOUBLED_STORAGE);
        assert_eq!(
            FeeVersion::get_optional(TEST_FEE_VERSION_NUMBER_DOUBLED_STORAGE),
            Some(&TEST_FEE_VERSION_DOUBLED_STORAGE)
        );

        let unregistered = (1 << TEST_PROTOCOL_VERSION_SHIFT_BYTES) + 2;
        assert!(
            FeeVersion::get(unregistered).is_err(),
            "a shifted number with no test generation is an error, not a fallback"
        );
        assert!(FeeVersion::get_optional(unregistered).is_none());

        assert!(
            FEE_VERSIONS
                .iter()
                .all(|fee_version| fee_version.fee_version_number
                    >> TEST_PROTOCOL_VERSION_SHIFT_BYTES
                    == 0),
            "the production registry must never carry a shifted number"
        );
    }

    #[test]
    fn should_keep_the_test_fee_generation_aligned_with_the_latest_schedule_except_storage_disk_usage(
    ) {
        let latest = &PlatformVersion::latest().fee_version;
        let doubled = &TEST_FEE_VERSION_DOUBLED_STORAGE;

        assert_eq!(
            doubled.storage.storage_disk_usage_credit_per_byte,
            2 * latest.storage.storage_disk_usage_credit_per_byte
        );

        // Everything but the generation number and the disk usage rate is the
        // latest schedule, so a fee difference across the boundary is storage.
        let aligned = FeeVersion {
            fee_version_number: latest.fee_version_number,
            storage: FeeStorageVersion {
                storage_disk_usage_credit_per_byte: latest
                    .storage
                    .storage_disk_usage_credit_per_byte,
                ..doubled.storage.clone()
            },
            ..doubled.clone()
        };
        assert_eq!(&aligned, latest);
    }
}
