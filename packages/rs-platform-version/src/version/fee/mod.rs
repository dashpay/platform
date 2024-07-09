use crate::version::fee::data_contract::FeeDataContractValidationVersion;
use crate::version::fee::hashing::FeeHashingVersion;
use crate::version::fee::processing::FeeProcessingVersion;
use crate::version::fee::signature::FeeSignatureVersion;
use crate::version::fee::state_transition_min_fees::StateTransitionMinFees;
use crate::version::fee::storage::FeeStorageVersion;
use bincode::{Decode, Encode};

mod data_contract;
mod hashing;
mod processing;
pub mod signature;
pub mod state_transition_min_fees;
pub mod storage;
pub mod v1;
pub mod v2;

#[derive(Clone, Debug, Encode, Decode, Default)]
pub struct FeeVersion {
    pub storage: FeeStorageVersion,
    pub signature: FeeSignatureVersion,
    pub hashing: FeeHashingVersion,
    pub processing: FeeProcessingVersion,
    pub data_contract: FeeDataContractValidationVersion,
    pub state_transition_min_fees: StateTransitionMinFees,
}

impl PartialEq for FeeVersion {
    fn eq(&self, other: &Self) -> bool {
        self.storage == other.storage
            && self.signature == other.signature
            && self.hashing == other.hashing
            && self.processing == other.processing
            && self.data_contract == other.data_contract
            && self.state_transition_min_fees == other.state_transition_min_fees
    }
}


#[cfg(test)]
mod tests {
    use crate::version::fee::data_contract::FeeDataContractValidationVersion;
    use crate::version::fee::hashing::FeeHashingVersion;
    use crate::version::fee::processing::FeeProcessingVersion;
    use crate::version::fee::signature::FeeSignatureVersion;
    use crate::version::fee::state_transition_min_fees::StateTransitionMinFees;
    use crate::version::fee::storage::FeeStorageVersion;
    use super::FeeVersion;

    #[test]
    // If this test failed, then a new field was added in FeeVersion. And the corresponding eq needs to be updated as well
    fn test_fee_version_equality() {
        let version1 = FeeVersion {
            storage: FeeStorageVersion {
                storage_disk_usage_credit_per_byte: 1,
                storage_processing_credit_per_byte: 2,
                storage_load_credit_per_byte: 3,
                non_storage_load_credit_per_byte: 4,
                storage_seek_cost: 5,
            },
            signature: FeeSignatureVersion {
                verify_signature_ecdsa_secp256k1: 1,
                verify_signature_bls12_381: 2,
                verify_signature_ecdsa_hash160: 3,
                verify_signature_bip13_script_hash: 4,
                verify_signature_eddsa25519_hash160: 5,
            },
            hashing: FeeHashingVersion {
                single_sha256_base: 1,
                blake3_base: 2,
                sha256_ripe_md160_base: 3,
                sha256_per_block: 4,
                blake3_per_block: 5,
            },
            processing:FeeProcessingVersion {
                fetch_identity_balance_processing_cost: 1,
                fetch_identity_revision_processing_cost: 2,
                fetch_identity_balance_and_revision_processing_cost: 3,
                fetch_identity_cost_per_look_up_key_by_id: 4,
                fetch_single_identity_key_processing_cost: 5,
                validate_key_structure: 6,
            },
            data_contract: FeeDataContractValidationVersion {
                document_type_base_fee: 1,
                document_type_size_fee: 2,
                document_type_per_property_fee: 3,
                document_type_base_non_unique_index_fee: 4,
                document_type_non_unique_index_per_property_fee: 5,
                document_type_base_unique_index_fee: 6,
                document_type_unique_index_per_property_fee: 7,
            },
            state_transition_min_fees: StateTransitionMinFees {
                credit_transfer: 1,
                credit_withdrawal: 2,
                identity_update: 3,
                document_batch_sub_transition: 4,
                contract_create: 5,
                contract_update: 6,
            }
        };

        let version2 = FeeVersion {
            storage: FeeStorageVersion {
                storage_disk_usage_credit_per_byte: 1,
                storage_processing_credit_per_byte: 2,
                storage_load_credit_per_byte: 3,
                non_storage_load_credit_per_byte: 4,
                storage_seek_cost: 5,
            },
            signature: FeeSignatureVersion {
                verify_signature_ecdsa_secp256k1: 1,
                verify_signature_bls12_381: 2,
                verify_signature_ecdsa_hash160: 3,
                verify_signature_bip13_script_hash: 4,
                verify_signature_eddsa25519_hash160: 5,
            },
            hashing: FeeHashingVersion {
                single_sha256_base: 1,
                blake3_base: 2,
                sha256_ripe_md160_base: 3,
                sha256_per_block: 4,
                blake3_per_block: 5,
            },
            processing:FeeProcessingVersion {
                fetch_identity_balance_processing_cost: 1,
                fetch_identity_revision_processing_cost: 2,
                fetch_identity_balance_and_revision_processing_cost: 3,
                fetch_identity_cost_per_look_up_key_by_id: 4,
                fetch_single_identity_key_processing_cost: 5,
                validate_key_structure: 6,
            },
            data_contract: FeeDataContractValidationVersion {
                document_type_base_fee: 1,
                document_type_size_fee: 2,
                document_type_per_property_fee: 3,
                document_type_base_non_unique_index_fee: 4,
                document_type_non_unique_index_per_property_fee: 5,
                document_type_base_unique_index_fee: 6,
                document_type_unique_index_per_property_fee: 7,
            },
            state_transition_min_fees: StateTransitionMinFees {
                credit_transfer: 1,
                credit_withdrawal: 2,
                identity_update: 3,
                document_batch_sub_transition: 4,
                contract_create: 5,
                contract_update: 6,
            }
        };

        // This assertion will check if all fields are considered in the equality comparison
        assert_eq!(version1, version2, "FeeVersion equality test failed. If a field was added or removed, update the Eq implementation.");
    }
}

