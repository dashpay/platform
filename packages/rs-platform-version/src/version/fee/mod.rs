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
pub mod v3;
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
/// The storage fee table exactly as every pre-4.2 release serialized it
/// (5 fields). `FeeStorageVersion` gained
/// `ttl_ephemeral_disk_usage_credit_per_byte` in 4.2; embedding the live
/// struct here would shift the frozen pre-1.4 platform-state wire format
/// and break deserialization of old stored states (bincode
/// `UnexpectedEnd`). Any future field added to `FeeStorageVersion` must
/// NOT be added here.
#[derive(Clone, Debug, Encode, Decode, Default, PartialEq, Eq)]
pub struct FeeStorageVersionFieldsBeforeVersion4 {
    pub storage_disk_usage_credit_per_byte: u64,
    pub storage_processing_credit_per_byte: u64,
    pub storage_load_credit_per_byte: u64,
    pub non_storage_load_credit_per_byte: u64,
    pub storage_seek_cost: u64,
}

impl From<FeeStorageVersionFieldsBeforeVersion4> for FeeStorageVersion {
    fn from(value: FeeStorageVersionFieldsBeforeVersion4) -> Self {
        FeeStorageVersion {
            storage_disk_usage_credit_per_byte: value.storage_disk_usage_credit_per_byte,
            storage_processing_credit_per_byte: value.storage_processing_credit_per_byte,
            storage_load_credit_per_byte: value.storage_load_credit_per_byte,
            non_storage_load_credit_per_byte: value.non_storage_load_credit_per_byte,
            storage_seek_cost: value.storage_seek_cost,
            // Pre-4.2 tables predate the TTL grammar entirely.
            ttl_ephemeral_disk_usage_credit_per_byte: 0,
        }
    }
}

#[derive(Clone, Debug, Encode, Decode, Default, PartialEq, Eq)]
pub struct FeeVersionFieldsBeforeVersion4 {
    // Permille means devise by 1000
    pub uses_version_fee_multiplier_permille: Option<u64>,
    pub storage: FeeStorageVersionFieldsBeforeVersion4,
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
            storage: value.storage.into(),
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
