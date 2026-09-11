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

/// The registry of fee-history generations, keyed by `fee_version_number`.
///
/// A fee-history generation is the set of values the persisted fee history can be asked for
/// through `KnownCostItem`: the storage, processing, hashing and signature groups. The epoch
/// change hook records the number of the active schedule in platform state, and saved state
/// stores that number, so every number that was ever recorded must stay registered here.
///
/// Entries are ordered by number, starting at 1 and without gaps. A schedule that changes any
/// value the fee history serves (storage rates in particular, because they price refunds) must
/// be registered under a new number. A schedule that only changes a group the history never
/// serves (for example `FEE_VERSION2`, which changed only `data_contract_registration`) keeps
/// the number of the generation it agrees with.
pub const FEE_VERSIONS: &[FeeVersion] = &[FEE_VERSION1];

const _: () = assert!(
    !FEE_VERSIONS.is_empty(),
    "the fee version registry must hold at least one generation"
);

const _: () = assert!(
    registry_numbers_are_contiguous_from_one(FEE_VERSIONS),
    "fee version numbers must be registered in order, starting at 1 and without gaps"
);

/// Returns true when the registry entries carry the numbers 1, 2, ... in order.
///
/// Evaluated at compile time so a mis-numbered entry cannot be built into a node.
const fn registry_numbers_are_contiguous_from_one(registry: &[FeeVersion]) -> bool {
    let mut index = 0;
    while index < registry.len() {
        if registry[index].fee_version_number != index as FeeVersionNumber + 1 {
            return false;
        }
        index += 1;
    }
    true
}

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
    /// Returns the registered fee-history generation this schedule's number names.
    ///
    /// The registered entry agrees with this schedule on every value the fee history serves,
    /// but it is not necessarily the same schedule constant: several schedules may share one
    /// number when they differ only in groups the history never reads. Fails when the number
    /// is not registered.
    pub fn as_static(&self) -> Result<&'static FeeVersion, PlatformVersionError> {
        FeeVersion::get(self.fee_version_number)
    }

    /// Resolves a fee version number to its registered generation.
    ///
    /// Lookup is by number, never by position in the registry. Zero and any unregistered number
    /// are errors.
    pub fn get<'a>(version: FeeVersionNumber) -> Result<&'a Self, PlatformVersionError> {
        FeeVersion::get_optional(version).ok_or_else(|| {
            PlatformVersionError::UnknownVersionError(format!("no fee version {version}"))
        })
    }

    /// Resolves a fee version number to its registered generation, or `None` when the number
    /// is zero or not registered.
    pub fn get_optional<'a>(version: FeeVersionNumber) -> Option<&'a Self> {
        FEE_VERSIONS
            .iter()
            .find(|fee_version| fee_version.fee_version_number == version)
    }

    /// The earliest registered generation. This is what an empty fee history resolves to.
    pub fn first<'a>() -> &'a Self {
        FEE_VERSIONS
            .first()
            .expect("the compile time assertion above proves the registry is not empty")
    }

    /// The most recently registered generation.
    pub fn latest<'a>() -> &'a Self {
        FEE_VERSIONS
            .last()
            .expect("the compile time assertion above proves the registry is not empty")
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
    #[cfg(feature = "mock-versions")]
    use crate::version::mocks::v2_test::TEST_PLATFORM_V2;
    #[cfg(feature = "mock-versions")]
    use crate::version::mocks::v3_test::TEST_PLATFORM_V3;
    use crate::version::PLATFORM_VERSIONS;

    /// Every schedule a platform version references, plus the mock schedules when they are
    /// compiled in.
    fn referenced_schedules() -> Vec<(&'static str, &'static FeeVersion)> {
        let shipped = PLATFORM_VERSIONS
            .iter()
            .map(|platform_version| ("shipped platform version", &platform_version.fee_version));
        #[cfg(feature = "mock-versions")]
        let mocks = [
            ("TEST_PLATFORM_V2", &TEST_PLATFORM_V2.fee_version),
            ("TEST_PLATFORM_V3", &TEST_PLATFORM_V3.fee_version),
        ];
        #[cfg(not(feature = "mock-versions"))]
        let mocks: [(&'static str, &'static FeeVersion); 0] = [];
        shipped.chain(mocks).collect()
    }

    #[test]
    fn should_resolve_every_registered_number_to_the_entry_with_that_number() {
        for registered in FEE_VERSIONS {
            let number = registered.fee_version_number;
            let resolved = FeeVersion::get(number).expect("registered number resolves");
            assert_eq!(
                resolved.fee_version_number, number,
                "lookup must return the entry carrying the requested number"
            );
            assert_eq!(resolved, registered);
            assert_eq!(FeeVersion::get_optional(number), Some(registered));
        }
    }

    #[test]
    fn should_reject_zero_and_unregistered_numbers() {
        let unregistered = [0, FEE_VERSIONS.len() as FeeVersionNumber + 1, u32::MAX];
        for number in unregistered {
            let error = FeeVersion::get(number).expect_err("unregistered number is an error");
            assert!(
                error.to_string().contains(&number.to_string()),
                "error names the unknown number: {error}"
            );
            assert!(FeeVersion::get_optional(number).is_none());
        }
    }

    #[test]
    fn should_keep_registered_numbers_unique_and_contiguous_from_one() {
        let numbers: Vec<FeeVersionNumber> = FEE_VERSIONS
            .iter()
            .map(|fee_version| fee_version.fee_version_number)
            .collect();
        let expected: Vec<FeeVersionNumber> =
            (1..=FEE_VERSIONS.len() as FeeVersionNumber).collect();
        assert_eq!(numbers, expected);
        assert!(registry_numbers_are_contiguous_from_one(FEE_VERSIONS));

        let skipped = [
            FEE_VERSION1,
            FeeVersion {
                fee_version_number: 3,
                ..FEE_VERSION1
            },
        ];
        assert!(!registry_numbers_are_contiguous_from_one(&skipped));
        let duplicated = [FEE_VERSION1, FEE_VERSION1];
        assert!(!registry_numbers_are_contiguous_from_one(&duplicated));
        let starting_at_zero = [FeeVersion {
            fee_version_number: 0,
            ..FEE_VERSION1
        }];
        assert!(!registry_numbers_are_contiguous_from_one(&starting_at_zero));
    }

    #[test]
    fn should_register_the_number_every_platform_version_references() {
        for (origin, schedule) in referenced_schedules() {
            let registered = FeeVersion::get(schedule.fee_version_number).unwrap_or_else(|error| {
                panic!(
                    "{origin} references fee version number {} which is not registered: {error}",
                    schedule.fee_version_number
                )
            });
            // The registered generation must agree with the schedule on every group the fee
            // history can serve. A schedule that changes one of these needs a new number.
            assert_eq!(
                registered.storage, schedule.storage,
                "{origin}: storage group"
            );
            assert_eq!(
                registered.processing, schedule.processing,
                "{origin}: processing group"
            );
            assert_eq!(
                registered.hashing, schedule.hashing,
                "{origin}: hashing group"
            );
            assert_eq!(
                registered.signature, schedule.signature,
                "{origin}: signature group"
            );
        }
    }

    #[test]
    fn should_keep_fee_version_one_storage_rates_frozen_for_replay() {
        // Every storage byte written on the network so far was priced with these rates, and
        // every refund of those bytes is priced with them again through the fee history.
        // Changing them under number 1 would re-price historical refunds during replay, so a
        // change in storage rates must be registered under a new number instead.
        let storage = &FeeVersion::get(1)
            .expect("fee version 1 is registered")
            .storage;
        assert_eq!(storage.storage_disk_usage_credit_per_byte, 27000);
        assert_eq!(storage.storage_processing_credit_per_byte, 400);
        assert_eq!(storage.storage_load_credit_per_byte, 20);
        assert_eq!(storage.non_storage_load_credit_per_byte, 10);
        assert_eq!(storage.storage_seek_cost, 2000);
    }

    #[test]
    fn should_return_the_registered_entry_from_as_static() {
        for (origin, schedule) in referenced_schedules() {
            let registered = schedule
                .as_static()
                .unwrap_or_else(|error| panic!("{origin}: {error}"));
            assert_eq!(registered.fee_version_number, schedule.fee_version_number);
            assert_eq!(
                registered,
                FeeVersion::get(schedule.fee_version_number).unwrap()
            );
        }
    }

    #[test]
    fn should_error_from_as_static_for_an_unregistered_number() {
        for number in [0, 99] {
            let schedule = FeeVersion {
                fee_version_number: number,
                ..FEE_VERSION1
            };
            let error = schedule
                .as_static()
                .expect_err("an unregistered number cannot resolve");
            assert!(error.to_string().contains(&number.to_string()));
        }
    }
}
