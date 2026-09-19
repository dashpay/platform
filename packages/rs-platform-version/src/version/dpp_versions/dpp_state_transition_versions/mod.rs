use versioned_feature_core::FeatureVersion;

pub mod v1;
pub mod v2;
pub mod v3;

#[derive(Clone, Debug, Default)]
pub struct DPPStateTransitionVersions {
    pub documents: DocumentTransitionVersions,
    pub identities: IdentityTransitionVersions,
    pub contract: ContractTransitionVersions,
    pub address_funds: AddressFundsTransitionVersions,
    pub max_address_inputs: u16,
    pub max_address_outputs: u16,
    pub max_address_fee_strategies: u16,
}

#[derive(Clone, Debug, Default)]
pub struct IdentityTransitionVersions {
    pub max_public_keys_in_creation: u16,
    pub asset_locks: IdentityTransitionAssetLockVersions,
    pub credit_withdrawal: IdentityCreditWithdrawalTransitionVersions,
    pub calculate_min_required_fee_on_identity_create_transition: FeatureVersion,
    pub calculate_min_required_fee_on_identity_top_up_transition: i32,
}

#[derive(Clone, Debug, Default)]
pub struct ContractTransitionVersions {
    pub contract_create_transition_default_version: FeatureVersion,
    pub contract_update_transition_default_version: FeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct AddressFundsTransitionVersions {
    pub address_funds_transition_default_version: FeatureVersion,
    pub credit_withdrawal: FeatureVersion,
    /// Minimum credits for an address output (500,000 credits = 0.000005 Dash)
    pub min_output_amount: u64,
    /// Minimum credits an input must contribute (100,000 credits = 0.000001 Dash)
    pub min_input_amount: u64,
    /// Minimum credits to fund an identity (from addresses)
    pub min_identity_funding_amount: u64,
}

#[derive(Clone, Debug, Default)]
pub struct IdentityCreditWithdrawalTransitionVersions {
    pub default_constructor: FeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct IdentityTransitionAssetLockVersions {
    pub required_asset_lock_duff_balance_for_processing_start_for_identity_create: u64,
    pub required_asset_lock_duff_balance_for_processing_start_for_identity_top_up: u64,
    pub required_asset_lock_duff_balance_for_processing_start_for_address_funding: u64,
    pub validate_asset_lock_transaction_structure: FeatureVersion,
    pub validate_instant_asset_lock_proof_structure: FeatureVersion,
    pub max_asset_lock_transaction_inputs: u16,
}

#[derive(Clone, Debug, Default)]
pub struct DocumentTransitionVersions {
    pub documents_batch_transition: DocumentsBatchTransitionVersions,
}

#[derive(Clone, Debug, Default)]
pub struct DocumentsBatchTransitionVersions {
    pub validation: DocumentsBatchTransitionValidationVersions,
}

#[derive(Clone, Debug, Default)]
pub struct DocumentsBatchTransitionValidationVersions {
    pub find_duplicates_by_id: FeatureVersion,
    pub validate_base_structure: FeatureVersion,
}
