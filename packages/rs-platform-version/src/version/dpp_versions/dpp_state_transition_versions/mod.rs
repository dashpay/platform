use versioned_feature_core::FeatureVersion;

pub mod v1;
pub mod v2;
pub mod v3;
pub mod v4;
pub mod v5;

#[derive(Clone, Debug, Default)]
pub struct DPPStateTransitionVersions {
    pub documents: DocumentTransitionVersions,
    pub identities: IdentityTransitionVersions,
    pub contract: ContractTransitionVersions,
    pub address_funds: AddressFundsTransitionVersions,
    pub shielded: ShieldedTransitionVersions,
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
    /// Stateless rules for the address credit withdrawal transition: 0 is the protocol version
    /// 13 rule set, 1 adds the Core fee-rate cap and the fee-inclusive minimum of version 14.
    pub validate_credit_withdrawal_structure: FeatureVersion,
    /// Minimum credits for an address output (500,000 credits = 0.000005 Dash)
    pub min_output_amount: u64,
    /// Minimum credits an input must contribute (100,000 credits = 0.000001 Dash)
    pub min_input_amount: u64,
    /// Minimum credits to fund an identity (from addresses)
    pub min_identity_funding_amount: u64,
}

#[derive(Clone, Debug, Default)]
pub struct ShieldedTransitionVersions {
    /// Stateless rules for the shielded withdrawal transition: 0 is the protocol version 13
    /// rule set, 1 adds the Core fee-rate cap and the fee-inclusive range check on the amount
    /// reserved for Core of version 14.
    pub validate_withdrawal_structure: FeatureVersion,
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
    /// Stateless checks of an asset lock transaction: payload, credit output, P2PKH. 0 also caps
    /// its inputs at `max_asset_lock_transaction_inputs`, a chain proof's transaction fetched
    /// from Core included; 1 (protocol 15) does not.
    pub validate_asset_lock_transaction_structure: FeatureVersion,
    /// Stateless checks of an instant asset lock proof. 0 relies on
    /// `validate_asset_lock_transaction_structure` 0 for the input cap, so a table selecting
    /// instant 0 must also select transaction 0; 1 (protocol 15) checks the cap itself.
    pub validate_instant_asset_lock_proof_structure: FeatureVersion,
    /// The most inputs the asset lock transaction an instant proof embeds may carry, so that the
    /// proof fits a state transition; before protocol 15 every asset lock transaction, a chain
    /// proof's included. Read by `validate_asset_lock_transaction_structure` 0 and by
    /// `validate_asset_lock_transaction_inputs`, which `validate_instant_asset_lock_proof_structure`
    /// 1 calls; wallets also read it as Dash Core's relay bound. `u16::MAX`, effectively no cap,
    /// before protocol version 11.
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
